//! Casos de uso de sólo lectura: lo que muestran las pantallas.

use chrono::Days;
use limen_dominio::busqueda::Criterio;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::Empresa;
use limen_dominio::empresa_proveedora::EmpresaProveedora;
use limen_dominio::gafete::TipoGafete;
use limen_dominio::personal_kof::PersonalKof;
use limen_dominio::praind::DIAS_ADVERTENCIA_PRAIND;

use crate::errores::ErrorCaso;
use crate::puertos::{
    Consultas, EntradaHistorial, FilaContratista, PersonaAdentro, RegistroAuditado, Reloj,
    ResumenGafete,
};

/// Cuántos resultados devuelve un buscador como máximo: una lista más larga
/// no se lee en la pantalla, y el operador afina lo que escribe.
pub const MAXIMO_RESULTADOS: usize = 50;

/// Las consultas no tienen reglas de negocio propias: no fallan por una
/// regla, sólo por una falla técnica.
pub type ErrorConsulta = ErrorCaso<std::convert::Infallible>;

fn limitar(pedido: usize) -> usize {
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

/// Todos los contratistas con el nombre de su empresa, para la grilla.
#[derive(Debug)]
pub struct ListarContratistas<C> {
    consultas: C,
}

impl<C: Consultas> ListarContratistas<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(&self) -> Result<Vec<FilaContratista>, ErrorConsulta> {
        Ok(self.consultas.listar_contratistas().await?)
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
