//! Los datos que cruzan entre la interfaz y los casos de uso, en JSON.
//!
//! El dominio no es serializable a propósito (arquitectura.md, 4.3): estos
//! tipos son la traducción. Convenciones, para que la interfaz no adivine:
//!
//! - los identificadores viajan como texto (UUID);
//! - las fechas, `AAAA-MM-DD`; los instantes, RFC 3339 en UTC;
//! - los tipos y las vías, con su código estable en mayúsculas
//!   (`PRAIND`, `IN_HOUSE`, `A_PIE`, `CONTRATISTA`…);
//! - los nombres de los campos, en `snake_case`.

use chrono::{NaiveDate, SecondsFormat};
use limen_aplicacion::casos_de_uso::consultas::ContratistaEnLista;
use limen_aplicacion::casos_de_uso::contratistas::ComandoContratista;
use limen_aplicacion::casos_de_uso::ingresos::{ComandoEntrada, EntradaContratista};
use limen_aplicacion::puertos::{IngresoAbierto, PersonaAdentro};
use limen_dominio::acceso::ResultadoAcceso;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::{Empresa, EmpresaId};
use limen_dominio::gafete::NumeroGafete;
use limen_dominio::medio::{Medio, TipoMedio};
use limen_dominio::presencia::Via;
use limen_dominio::tipo_ingreso::TipoIngreso;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ErrorEntrada;

const A_PIE: &str = "A_PIE";
const VEHICULO: &str = "VEHICULO";

pub fn leer_uuid(texto: &str) -> Result<Uuid, ErrorEntrada> {
    Uuid::parse_str(texto.trim()).map_err(|_| ErrorEntrada::IdInvalido)
}

pub fn leer_via(codigo: &str) -> Result<Via, ErrorEntrada> {
    Via::desde_codigo(codigo).ok_or(ErrorEntrada::ViaInvalida)
}

fn leer_medio(codigo: &str) -> Result<TipoMedio, ErrorEntrada> {
    match codigo {
        A_PIE => Ok(TipoMedio::APie),
        VEHICULO => Ok(TipoMedio::Vehiculo),
        _ => Err(ErrorEntrada::MedioInvalido),
    }
}

// --- Salida: lo que la interfaz muestra ---

/// Una persona que está adentro, por cualquiera de las cuatro vías.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonaAdentroDto {
    /// `CONTRATISTA`, `PROVEEDOR`, `CORREO` o `KOF`.
    pub via: &'static str,
    /// El ingreso abierto: con él se registra la salida.
    pub ingreso_id: String,
    /// La cédula o, para el personal KOF, el código de empleado.
    pub identidad: String,
    pub nombre: String,
    pub procedencia: String,
    /// `A_PIE` o `VEHICULO`; el personal KOF no lo registra.
    pub medio: Option<&'static str>,
    pub placa: Option<String>,
    pub gafete: Option<u32>,
    /// Desde cuándo está adentro (RFC 3339, UTC).
    pub desde: String,
}

impl From<&PersonaAdentro> for PersonaAdentroDto {
    fn from(persona: &PersonaAdentro) -> Self {
        let ingreso_id = match persona.ingreso {
            IngresoAbierto::Contratista(id) => id.uuid(),
            IngresoAbierto::Proveedor(id) => id.uuid(),
            IngresoAbierto::Correo(id) => id.uuid(),
            IngresoAbierto::Kof(id) => id.uuid(),
        };
        let (medio, placa) = match &persona.medio {
            None => (None, None),
            Some(Medio::APie) => (Some(A_PIE), None),
            Some(Medio::Vehiculo(placa)) => (Some(VEHICULO), Some(placa.as_str().to_owned())),
        };
        Self {
            via: persona.ingreso.via().codigo(),
            ingreso_id: ingreso_id.to_string(),
            identidad: persona.identidad.to_string(),
            nombre: persona.nombre.as_str().to_owned(),
            procedencia: persona.procedencia.clone(),
            medio,
            placa,
            gafete: persona.gafete.map(NumeroGafete::valor),
            desde: persona.desde.to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmpresaDto {
    pub id: String,
    pub nombre: String,
}

impl From<&Empresa> for EmpresaDto {
    fn from(empresa: &Empresa) -> Self {
        Self {
            id: empresa.id().uuid().to_string(),
            nombre: empresa.nombre().as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContratistaDto {
    pub id: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: String,
    /// `PRAIND` o `IN_HOUSE`.
    pub tipo_ingreso: &'static str,
    /// `AAAA-MM-DD`.
    pub fecha_vencimiento_praind: String,
    pub tiene_acceso: bool,
    /// Si lleva gafete físico (regla B10): la pantalla sólo lo muestra.
    pub requiere_gafete: bool,
}

impl From<&Contratista> for ContratistaDto {
    fn from(contratista: &Contratista) -> Self {
        Self {
            id: contratista.id().uuid().to_string(),
            cedula: contratista.cedula().as_str().to_owned(),
            nombre: contratista.nombre().as_str().to_owned(),
            empresa_id: contratista.empresa().uuid().to_string(),
            tipo_ingreso: contratista.tipo_ingreso().codigo(),
            fecha_vencimiento_praind: contratista.fecha_vencimiento_praind().to_string(),
            tiene_acceso: contratista.tiene_acceso(),
            requiere_gafete: contratista.requiere_gafete(),
        }
    }
}

/// Una fila de la grilla de contratistas: el contratista, el nombre de su
/// empresa y lo que el dominio decide hoy sobre su acceso (la pantalla sólo
/// lo muestra).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilaContratistaDto {
    #[serde(flatten)]
    pub contratista: ContratistaDto,
    pub empresa_nombre: String,
    pub acceso: AccesoDto,
}

impl From<&ContratistaEnLista> for FilaContratistaDto {
    fn from(en_lista: &ContratistaEnLista) -> Self {
        Self {
            contratista: ContratistaDto::from(&en_lista.fila.contratista),
            empresa_nombre: en_lista.fila.empresa.as_str().to_owned(),
            acceso: en_lista.acceso.into(),
        }
    }
}

/// Qué decidió el dominio sobre el acceso al entrar (regla D). Si el acceso
/// se deniega, la entrada no se registra y el comando devuelve el error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccesoDto {
    /// `PERMITIDO`, `PERMITIDO_CON_ADVERTENCIA` o `DENEGADO`.
    pub resultado: &'static str,
    /// Con advertencia: cuántos días le quedan al PRAIND (0 = vence hoy).
    pub dias_para_vencer: Option<i64>,
    /// Denegado: el código del motivo.
    pub motivo: Option<&'static str>,
}

impl From<ResultadoAcceso> for AccesoDto {
    fn from(acceso: ResultadoAcceso) -> Self {
        match acceso {
            ResultadoAcceso::Permitido => Self {
                resultado: "PERMITIDO",
                dias_para_vencer: None,
                motivo: None,
            },
            ResultadoAcceso::PermitidoConAdvertencia { dias_para_vencer } => Self {
                resultado: "PERMITIDO_CON_ADVERTENCIA",
                dias_para_vencer: Some(dias_para_vencer),
                motivo: None,
            },
            ResultadoAcceso::Denegado(motivo) => Self {
                resultado: "DENEGADO",
                dias_para_vencer: None,
                motivo: Some(motivo.codigo()),
            },
        }
    }
}

/// La entrada de un contratista ya registrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntradaRegistradaDto {
    pub ingreso_id: String,
    pub acceso: AccesoDto,
}

impl From<EntradaContratista> for EntradaRegistradaDto {
    fn from(entrada: EntradaContratista) -> Self {
        Self {
            ingreso_id: entrada.ingreso.uuid().to_string(),
            acceso: entrada.acceso.into(),
        }
    }
}

// --- Entrada: lo que llena el operador ---

/// El formulario de un contratista, tal cual se escribió. Las reglas (cédula,
/// nombre, PRAIND) las aplica el dominio; aquí sólo se lee el formato.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ContratistaEntrada {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: String,
    /// `PRAIND` o `IN_HOUSE`.
    pub tipo_ingreso: String,
    /// `AAAA-MM-DD`.
    pub fecha_vencimiento_praind: String,
    pub tiene_acceso: bool,
}

impl ContratistaEntrada {
    pub fn a_comando(&self) -> Result<ComandoContratista, ErrorEntrada> {
        Ok(ComandoContratista {
            cedula: self.cedula.clone(),
            nombre: self.nombre.clone(),
            empresa: EmpresaId::desde_uuid(leer_uuid(&self.empresa_id)?),
            tipo_ingreso: TipoIngreso::desde_codigo(&self.tipo_ingreso)
                .map_err(|_| ErrorEntrada::TipoIngresoInvalido)?,
            fecha_vencimiento_praind: NaiveDate::parse_from_str(
                self.fecha_vencimiento_praind.trim(),
                "%Y-%m-%d",
            )
            .map_err(|_| ErrorEntrada::FechaInvalida)?,
            tiene_acceso: self.tiene_acceso,
        })
    }
}

/// El formulario de entrada de un contratista.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EntradaContratistaEntrada {
    pub contratista_id: String,
    /// `A_PIE` o `VEHICULO`.
    pub medio: String,
    pub placa: Option<String>,
    /// Número del gafete; vacío = "sin gafete".
    pub gafete: Option<u32>,
}

impl EntradaContratistaEntrada {
    pub fn a_comando(&self) -> Result<ComandoEntrada, ErrorEntrada> {
        Ok(ComandoEntrada {
            contratista: limen_dominio::contratista::ContratistaId::desde_uuid(leer_uuid(
                &self.contratista_id,
            )?),
            medio: leer_medio(&self.medio)?,
            placa: self.placa.clone(),
            gafete: self.gafete,
        })
    }
}
