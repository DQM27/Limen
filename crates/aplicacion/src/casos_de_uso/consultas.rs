//! Casos de uso de sólo lectura: lo que muestran las pantallas.

use std::collections::HashMap;

use chrono::{Days, NaiveDate};
use limen_dominio::acceso::{ResultadoAcceso, verificar_acceso};
use limen_dominio::busqueda::Criterio;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::Empresa;
use limen_dominio::empresa_proveedora::EmpresaProveedora;
use limen_dominio::gafete::{NumeroGafete, Portador, TipoGafete};
use limen_dominio::nombre::NombrePersona;
use limen_dominio::permanencia::lleva_demasiado_adentro;
use limen_dominio::personal_kof::PersonalKof;
use limen_dominio::praind::DIAS_ADVERTENCIA_PRAIND;
use limen_dominio::rango_fechas::{Atajo, ErrorRango, RangoFechas};

use crate::errores::ErrorCaso;
use crate::puertos::{
    Consultas, EntradaHistorial, FabricaUnidadDeTrabajo, FilaContratista, MovimientoHistorial,
    PersonaAdentro, RegistroAuditado, Reloj, RepositorioContratistas, RepositorioPersonalKof,
    ResumenGafete, UnidadDeTrabajo,
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

/// Una persona que está adentro y lo que el dominio decide sobre su
/// permanencia: la pantalla sólo lo muestra, no lo calcula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaAdentroVista {
    pub persona: PersonaAdentro,
    /// Lleva más de 12 horas adentro (E13): probablemente falta registrar su
    /// salida.
    pub demasiado_tiempo: bool,
}

#[derive(Debug)]
pub struct QuienesEstanAdentro<C, R> {
    consultas: C,
    reloj: R,
}

impl<C: Consultas, R: Reloj> QuienesEstanAdentro<C, R> {
    pub const fn new(consultas: C, reloj: R) -> Self {
        Self { consultas, reloj }
    }

    pub async fn ejecutar(&self) -> Result<Vec<PersonaAdentro>, ErrorConsulta> {
        Ok(self.consultas.quienes_estan_adentro().await?)
    }

    /// Lo mismo, con la marca de quien lleva demasiado tiempo adentro.
    pub async fn con_permanencia(&self) -> Result<Vec<PersonaAdentroVista>, ErrorConsulta> {
        let ahora = self.reloj.ahora();
        Ok(self
            .consultas
            .quienes_estan_adentro()
            .await?
            .into_iter()
            .map(|persona| PersonaAdentroVista {
                demasiado_tiempo: lleva_demasiado_adentro(persona.entrada.en, ahora),
                persona,
            })
            .collect())
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

/// A quién está asignado un gafete: quien lo tiene prestado ahora o, si está
/// perdido, quien lo tenía.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsignadoGafete {
    /// `None` cuando sólo se conoce la cédula: un proveedor o una visita que
    /// perdió el gafete no están en ningún catálogo (F5).
    pub nombre: Option<NombrePersona>,
    /// La cédula o, para el personal KOF, el código de empleado.
    pub identidad: String,
}

/// Un gafete del catálogo, con su estado, si está prestado y a quién.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafeteListado {
    pub resumen: ResumenGafete,
    pub asignado: Option<AsignadoGafete>,
}

/// Los gafetes de un tipo con su estado, si están prestados y a quién
/// pertenecen ahora (prestado, o perdido con su último portador).
#[derive(Debug)]
pub struct ListarGafetes<F> {
    fabrica: F,
}

impl<F: FabricaUnidadDeTrabajo + Consultas> ListarGafetes<F> {
    pub const fn new(fabrica: F) -> Self {
        Self { fabrica }
    }

    pub async fn ejecutar(&self, tipo: TipoGafete) -> Result<Vec<GafeteListado>, ErrorConsulta> {
        let resumenes = self.fabrica.listar_gafetes(tipo).await?;
        // Quién tiene cada gafete prestado: sale de quién está adentro por la
        // vía de ese tipo (F2).
        let prestados: HashMap<NumeroGafete, AsignadoGafete> =
            if resumenes.iter().any(|r| r.prestado) {
                self.fabrica
                    .quienes_estan_adentro()
                    .await?
                    .into_iter()
                    .filter(|persona| persona.ingreso.via() == tipo.via())
                    .filter_map(|persona| {
                        persona.gafete.map(|numero| {
                            (
                                numero,
                                AsignadoGafete {
                                    nombre: Some(persona.nombre),
                                    identidad: persona.identidad.to_string(),
                                },
                            )
                        })
                    })
                    .collect()
            } else {
                HashMap::new()
            };
        let mut listado = Vec::with_capacity(resumenes.len());
        for resumen in resumenes {
            let asignado = if resumen.prestado {
                prestados.get(&resumen.gafete.numero()).cloned()
            } else {
                match resumen.gafete.portador() {
                    Some(portador) => Some(self.ultimo_portador(portador).await?),
                    None => None,
                }
            };
            listado.push(GafeteListado { resumen, asignado });
        }
        Ok(listado)
    }

    /// El último portador de un gafete perdido, con su nombre si está en un
    /// catálogo. Si ya no se encuentra, queda sólo su identificación.
    async fn ultimo_portador(&self, portador: &Portador) -> Result<AsignadoGafete, ErrorConsulta> {
        let mut uow = self.fabrica.nueva();
        Ok(match portador {
            Portador::Persona(cedula) => AsignadoGafete {
                nombre: None,
                identidad: cedula.to_string(),
            },
            Portador::Contratista(id) => {
                let contratista = uow.contratistas().obtener(*id).await?;
                AsignadoGafete {
                    nombre: contratista.as_ref().map(|c| c.nombre().clone()),
                    identidad: contratista.map_or_else(String::new, |c| c.cedula().to_string()),
                }
            }
            Portador::PersonalKof(id) => {
                let persona = uow.personal_kof().obtener(*id).await?;
                AsignadoGafete {
                    nombre: persona.as_ref().map(|p| p.nombre().clone()),
                    identidad: persona.map_or_else(String::new, |p| p.codigo().to_string()),
                }
            }
        })
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
