//! Casos de uso de sólo lectura: lo que muestran las pantallas.

use chrono::{Days, NaiveDate};
use limen_dominio::acceso::{ResultadoAcceso, verificar_acceso};
use limen_dominio::busqueda::Criterio;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::Empresa;
use limen_dominio::empresa_proveedora::EmpresaProveedora;
use limen_dominio::gafete::TipoGafete;
use limen_dominio::personal_kof::PersonalKof;
use limen_dominio::praind::DIAS_ADVERTENCIA_PRAIND;
use limen_dominio::rango_fechas::{Atajo, ErrorRango, RangoFechas};

use crate::errores::ErrorCaso;
use crate::puertos::{
    Consultas, EntradaHistorial, FilaContratista, MovimientoHistorial, PersonaAdentro,
    RegistroAuditado, Reloj, ResumenGafete,
};

/// Cuántos resultados devuelve un buscador como máximo: una lista más larga
/// no se lee en la pantalla, y el operador afina lo que escribe.
pub const MAXIMO_RESULTADOS: usize = 50;

/// Las consultas no tienen reglas de negocio propias: no fallan por una
/// regla, sólo por una falla técnica.
pub type ErrorConsulta = ErrorCaso<std::convert::Infallible>;

pub(crate) fn limitar(pedido: usize) -> usize {
    pedido.clamp(1, MAXIMO_RESULTADOS)
}

#[derive(Debug)]
pub struct QuienesEstanAdentro<C> {
    consultas: C,
}

impl<C: Consultas> QuienesEstanAdentro<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(&self) -> Result<Vec<PersonaAdentro>, ErrorConsulta> {
        Ok(self.consultas.quienes_estan_adentro().await?)
    }
}

/// Una fila de la grilla de contratistas y lo que el dominio decide hoy
/// sobre su acceso (regla D): la pantalla sólo lo muestra, no lo calcula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContratistaEnLista {
    pub fila: FilaContratista,
    pub acceso: ResultadoAcceso,
}

/// Cuántos movimientos trae el historial como máximo: más no se leen en una
/// pantalla, y quien necesite todo acota las fechas.
pub const MAXIMO_MOVIMIENTOS: usize = 20_000;

pub type ErrorHistorial = ErrorCaso<ErrorRango>;

/// Los movimientos de un rango de fechas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Historial {
    /// El rango que se consultó (con los extremos abiertos, si lo estaban).
    pub rango: RangoFechas,
    /// Del más reciente al más antiguo.
    pub movimientos: Vec<MovimientoHistorial>,
    /// `true` si el rango tenía más de [`MAXIMO_MOVIMIENTOS`]: se muestran los
    /// más recientes.
    pub truncado: bool,
}

/// El historial de ingresos y salidas de las cuatro vías en un rango de
/// fechas. Las fechas son días de Costa Rica; el día final se incluye
/// completo. El rango lo valida el dominio.
#[derive(Debug)]
pub struct ListarHistorial<C, R> {
    consultas: C,
    reloj: R,
}

impl<C: Consultas, R: Reloj> ListarHistorial<C, R> {
    pub const fn new(consultas: C, reloj: R) -> Self {
        Self { consultas, reloj }
    }

    pub async fn ejecutar(
        &self,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<Historial, ErrorHistorial> {
        let rango = RangoFechas::nuevo(desde, hasta).map_err(ErrorCaso::Negocio)?;
        let desde_utc = rango.desde().map(|dia| self.reloj.inicio_del_dia(dia));
        // El día final entra completo: el límite es el inicio del día siguiente.
        let hasta_utc = rango
            .hasta()
            .and_then(|dia| dia.checked_add_days(Days::new(1)))
            .map(|dia| self.reloj.inicio_del_dia(dia));
        // Se pide uno de más para saber si el rango se pasó del tope.
        let mut movimientos = self
            .consultas
            .historial_de_ingresos(desde_utc, hasta_utc, MAXIMO_MOVIMIENTOS + 1)
            .await?;
        let truncado = movimientos.len() > MAXIMO_MOVIMIENTOS;
        movimientos.truncate(MAXIMO_MOVIMIENTOS);
        Ok(Historial {
            rango,
            movimientos,
            truncado,
        })
    }
}

/// Un acceso rápido de fecha y el rango que da hoy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtajoConRango {
    pub atajo: Atajo,
    pub rango: RangoFechas,
}

/// Los accesos rápidos de fecha (hoy, esta semana…) con el rango que cada uno
/// da según la fecha de hoy en Costa Rica.
#[derive(Debug)]
pub struct AtajosDeFecha<R> {
    reloj: R,
}

impl<R: Reloj> AtajosDeFecha<R> {
    pub const fn new(reloj: R) -> Self {
        Self { reloj }
    }

    pub fn ejecutar(&self) -> Vec<AtajoConRango> {
        let hoy = self.reloj.hoy();
        Atajo::TODOS
            .into_iter()
            .map(|atajo| AtajoConRango {
                atajo,
                rango: atajo.rango(hoy),
            })
            .collect()
    }
}

/// Todos los contratistas con el nombre de su empresa y su estado de acceso
/// de hoy, para la grilla.
#[derive(Debug)]
pub struct ListarContratistas<C, R> {
    consultas: C,
    reloj: R,
}

impl<C: Consultas, R: Reloj> ListarContratistas<C, R> {
    pub const fn new(consultas: C, reloj: R) -> Self {
        Self { consultas, reloj }
    }

    pub async fn ejecutar(&self) -> Result<Vec<ContratistaEnLista>, ErrorConsulta> {
        let hoy = self.reloj.hoy();
        let filas = self.consultas.listar_contratistas().await?;
        Ok(filas
            .into_iter()
            .map(|fila| ContratistaEnLista {
                acceso: verificar_acceso(&fila.contratista, hoy),
                fila,
            })
            .collect())
    }
}

/// Busca contratistas por cédula (el inicio) o por nombre. Un texto vacío
/// no devuelve nada.
#[derive(Debug)]
pub struct BuscarContratistas<C> {
    consultas: C,
}

impl<C: Consultas> BuscarContratistas<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<Contratista>, ErrorConsulta> {
        let criterio = Criterio::desde_texto(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_contratistas(&criterio, limitar(limite))
            .await?)
    }
}

/// Busca personal KOF por código de empleado (el inicio) o por nombre.
#[derive(Debug)]
pub struct BuscarPersonalKof<C> {
    consultas: C,
}

impl<C: Consultas> BuscarPersonalKof<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<PersonalKof>, ErrorConsulta> {
        let criterio = Criterio::desde_texto(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_personal_kof(&criterio, limitar(limite))
            .await?)
    }
}

/// Busca empresas de contratistas por nombre (también si se escriben
/// números: "3M").
#[derive(Debug)]
pub struct BuscarEmpresas<C> {
    consultas: C,
}

impl<C: Consultas> BuscarEmpresas<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<Empresa>, ErrorConsulta> {
        let criterio = Criterio::de_nombre(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_empresas(&criterio, limitar(limite))
            .await?)
    }
}

/// Busca empresas proveedoras por nombre.
#[derive(Debug)]
pub struct BuscarEmpresasProveedoras<C> {
    consultas: C,
}

impl<C: Consultas> BuscarEmpresasProveedoras<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<EmpresaProveedora>, ErrorConsulta> {
        let criterio = Criterio::de_nombre(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_empresas_proveedoras(&criterio, limitar(limite))
            .await?)
    }
}

/// El historial de cambios de un registro, del más antiguo al más reciente.
#[derive(Debug)]
pub struct HistorialDeCambios<C> {
    consultas: C,
}

impl<C: Consultas> HistorialDeCambios<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        registro: RegistroAuditado,
    ) -> Result<Vec<EntradaHistorial>, ErrorConsulta> {
        Ok(self.consultas.historial_de(registro).await?)
    }
}

/// Los gafetes de un tipo con su estado y si están prestados.
#[derive(Debug)]
pub struct ListarGafetes<C> {
    consultas: C,
}

impl<C: Consultas> ListarGafetes<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(&self, tipo: TipoGafete) -> Result<Vec<ResumenGafete>, ErrorConsulta> {
        Ok(self.consultas.listar_gafetes(tipo).await?)
    }
}

/// Cuántos contratistas con el PRAIND por vencer muestra la lista.
pub const MAXIMO_PRAIND_POR_VENCER: usize = 200;

/// Contratistas con acceso cuyo PRAIND ya venció o vence dentro del plazo de
/// advertencia (regla D: 30 días), del que vence primero al último.
#[derive(Debug)]
pub struct PraindPorVencer<C, R> {
    consultas: C,
    reloj: R,
}

impl<C: Consultas, R: Reloj> PraindPorVencer<C, R> {
    pub const fn new(consultas: C, reloj: R) -> Self {
        Self { consultas, reloj }
    }

    pub async fn ejecutar(&self) -> Result<Vec<Contratista>, ErrorConsulta> {
        let hoy = self.reloj.hoy();
        let hasta = hoy
            .checked_add_days(Days::new(DIAS_ADVERTENCIA_PRAIND.unsigned_abs()))
            .unwrap_or(hoy);
        Ok(self
            .consultas
            .contratistas_con_praind_hasta(hasta, MAXIMO_PRAIND_POR_VENCER)
            .await?)
    }
}
