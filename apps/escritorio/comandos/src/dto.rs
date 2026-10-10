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
use limen_aplicacion::casos_de_uso::correo::ComandoEntradaCorreo;
use limen_aplicacion::casos_de_uso::gafetes::CambioGafete;
use limen_aplicacion::casos_de_uso::ingresos::{ComandoEntrada, EntradaContratista};
use limen_aplicacion::casos_de_uso::proveedores::ComandoEntradaProveedor;
use limen_aplicacion::puertos::{IngresoAbierto, PersonaAdentro, ResumenGafete};
use limen_dominio::acceso::ResultadoAcceso;
use limen_dominio::auditoria::CambioCampo;
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{Deudor, NumeroGafete, Resolucion, TipoGafete};
use limen_dominio::medio::{Medio, TipoMedio};
use limen_dominio::personal_kof::PersonalKof;
use limen_dominio::presencia::Via;
use limen_dominio::tipo_ingreso::TipoIngreso;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::campos;
use crate::error::{ErrorEntrada, ErrorJson};

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
    /// Lee el formato de cada campo; si uno no se puede leer, el error dice
    /// cuál es.
    pub fn a_comando(&self) -> Result<ComandoContratista, ErrorJson> {
        let en = |campo: &'static str| {
            move |error: ErrorEntrada| ErrorJson::from(error).en_campo(Some(campo))
        };
        Ok(ComandoContratista {
            cedula: self.cedula.clone(),
            nombre: self.nombre.clone(),
            empresa: EmpresaId::desde_uuid(
                leer_uuid(&self.empresa_id).map_err(en(campos::EMPRESA))?,
            ),
            tipo_ingreso: TipoIngreso::desde_codigo(&self.tipo_ingreso)
                .map_err(|_| ErrorEntrada::TipoIngresoInvalido)
                .map_err(en(campos::TIPO_INGRESO))?,
            fecha_vencimiento_praind: NaiveDate::parse_from_str(
                self.fecha_vencimiento_praind.trim(),
                "%Y-%m-%d",
            )
            .map_err(|_| ErrorEntrada::FechaInvalida)
            .map_err(en(campos::FECHA_VENCIMIENTO_PRAIND))?,
            tiene_acceso: self.tiene_acceso,
        })
    }
}

/// Un campo que cambió al editar (lo decide el dominio, regla B11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CambioDto {
    pub campo: &'static str,
    pub antes: String,
    pub despues: String,
}

impl From<&CambioCampo> for CambioDto {
    fn from(cambio: &CambioCampo) -> Self {
        Self {
            campo: cambio.campo,
            antes: cambio.antes.clone(),
            despues: cambio.despues.clone(),
        }
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
            contratista: ContratistaId::desde_uuid(leer_uuid(&self.contratista_id)?),
            medio: leer_medio(&self.medio)?,
            placa: self.placa.clone(),
            gafete: self.gafete,
        })
    }
}

// --- Proveedores, correo, personal KOF y gafetes ---

/// Lee el tipo de gafete por su código (`CONTRATISTA`, `VISITA`,
/// `PROVEEDOR`, `PROVISIONAL_KOF`).
pub fn leer_tipo_gafete(codigo: &str) -> Result<TipoGafete, ErrorEntrada> {
    TipoGafete::desde_codigo(codigo).ok_or(ErrorEntrada::TipoGafeteInvalido)
}

/// Un error de lectura atribuido a su campo.
fn en(campo: &'static str) -> impl Fn(ErrorEntrada) -> ErrorJson {
    move |error| ErrorJson::from(error).en_campo(Some(campo))
}

/// El formulario de entrada de un proveedor. Cédula y nombre llegan en
/// cada ingreso (no hay catálogo de personas, regla H2).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EntradaProveedorEntrada {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: String,
    /// `A_PIE` o `VEHICULO`.
    pub medio: String,
    pub placa: Option<String>,
    /// El gafete de proveedor es obligatorio (H3).
    pub gafete: u32,
}

impl EntradaProveedorEntrada {
    pub fn a_comando(&self) -> Result<ComandoEntradaProveedor, ErrorJson> {
        Ok(ComandoEntradaProveedor {
            cedula: self.cedula.clone(),
            nombre: self.nombre.clone(),
            empresa: EmpresaProveedoraId::desde_uuid(
                leer_uuid(&self.empresa_id).map_err(en(campos::EMPRESA))?,
            ),
            medio: leer_medio(&self.medio).map_err(en(campos::MEDIO))?,
            placa: self.placa.clone(),
            gafete: self.gafete,
        })
    }
}

/// El formulario de un ingreso por correo (visita autorizada).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EntradaCorreoEntrada {
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    /// `A_PIE` o `VEHICULO`.
    pub medio: String,
    pub placa: Option<String>,
    /// El gafete de visita es obligatorio (I2).
    pub gafete: u32,
}

impl EntradaCorreoEntrada {
    pub fn a_comando(&self) -> Result<ComandoEntradaCorreo, ErrorJson> {
        Ok(ComandoEntradaCorreo {
            cedula: self.cedula.clone(),
            nombre: self.nombre.clone(),
            motivo: self.motivo.clone(),
            medio: leer_medio(&self.medio).map_err(en(campos::MEDIO))?,
            placa: self.placa.clone(),
            gafete: self.gafete,
        })
    }
}

/// Una empresa proveedora (catálogo aparte de las de contratistas, H1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmpresaProveedoraDto {
    pub id: String,
    pub nombre: String,
}

impl From<&EmpresaProveedora> for EmpresaProveedoraDto {
    fn from(empresa: &EmpresaProveedora) -> Self {
        Self {
            id: empresa.id().uuid().to_string(),
            nombre: empresa.nombre().as_str().to_owned(),
        }
    }
}

/// Una persona del personal KOF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonalKofDto {
    pub id: String,
    pub codigo_empleado: String,
    pub nombre: String,
    /// Se desactiva, no se borra (K2).
    pub activo: bool,
}

impl From<&PersonalKof> for PersonalKofDto {
    fn from(persona: &PersonalKof) -> Self {
        Self {
            id: persona.id().uuid().to_string(),
            codigo_empleado: persona.codigo().as_str().to_owned(),
            nombre: persona.nombre().as_str().to_owned(),
            activo: persona.activo(),
        }
    }
}

/// Un gafete del catálogo, con su estado y si está prestado ahora.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GafeteDto {
    /// `CONTRATISTA`, `VISITA`, `PROVEEDOR` o `PROVISIONAL_KOF`.
    pub tipo: &'static str,
    pub numero: u32,
    /// `DISPONIBLE`, `PERDIDO` o `DE_BAJA`.
    pub estado: &'static str,
    pub prestado: bool,
    /// Si está perdido y lo debe un contratista: su identificador.
    pub deudor_contratista_id: Option<String>,
    /// Si está perdido y lo debe un proveedor o una visita: su cédula.
    pub deudor_cedula: Option<String>,
}

impl From<&ResumenGafete> for GafeteDto {
    fn from(resumen: &ResumenGafete) -> Self {
        let gafete = &resumen.gafete;
        let (deudor_contratista_id, deudor_cedula) = match gafete.deudor() {
            Some(Deudor::Contratista(id)) => (Some(id.uuid().to_string()), None),
            Some(Deudor::Persona(cedula)) => (None, Some(cedula.as_str().to_owned())),
            None => (None, None),
        };
        Self {
            tipo: gafete.tipo().codigo(),
            numero: gafete.numero().valor(),
            estado: gafete.estado().codigo(),
            prestado: resumen.prestado,
            deudor_contratista_id,
            deudor_cedula,
        }
    }
}

/// Un cambio de estado de un gafete (reglas F3 a F6).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CambioGafeteEntrada {
    /// `PERDIDO`, `PAGADO`, `APARECIDO` o `DE_BAJA`.
    pub cambio: String,
    /// Al marcarlo perdido, quién lo debe (F5): un contratista…
    pub deudor_contratista_id: Option<String>,
    /// …o la cédula de un proveedor o una visita. Sólo uno de los dos.
    pub deudor_cedula: Option<String>,
}

impl CambioGafeteEntrada {
    pub fn a_cambio(&self) -> Result<CambioGafete, ErrorJson> {
        match self.cambio.as_str() {
            "PERDIDO" => Ok(CambioGafete::MarcarPerdido(self.deudor()?)),
            "PAGADO" => Ok(CambioGafete::Resolver(Resolucion::Pagado)),
            "APARECIDO" => Ok(CambioGafete::Resolver(Resolucion::Aparecido)),
            "DE_BAJA" => Ok(CambioGafete::DarDeBaja),
            _ => Err(ErrorJson::from(ErrorEntrada::CambioGafeteInvalido).en_campo(Some("cambio"))),
        }
    }

    fn deudor(&self) -> Result<Deudor, ErrorJson> {
        let invalido =
            || ErrorJson::from(ErrorEntrada::DeudorInvalido).en_campo(Some(campos::DEUDOR));
        match (&self.deudor_contratista_id, &self.deudor_cedula) {
            (Some(id), None) => Ok(Deudor::Contratista(ContratistaId::desde_uuid(
                leer_uuid(id).map_err(|_| invalido())?,
            ))),
            (None, Some(cedula)) => Cedula::normalizar(cedula)
                .map(Deudor::Persona)
                .map_err(|_| invalido()),
            _ => Err(invalido()),
        }
    }
}
