//! Modelos de persistencia: cómo se guarda cada entidad en `SurrealDB`, y
//! su conversión hacia y desde el dominio.
//!
//! Son structs distintos de las entidades a propósito: el dominio no conoce
//! `SurrealDB`, y la forma de guardar puede cambiar sin tocar las reglas.

use chrono::{DateTime, NaiveDate, Utc};
use limen_aplicacion::puertos::{EntradaAuditoria, ErrorPersistencia};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaGuardado, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{Deudor, EstadoGafete, Gafete, NumeroGafete, TipoGafete};
use limen_dominio::ingreso_contratista::{IngresoContratista, IngresoGuardado, IngresoId};
use limen_dominio::ingreso_proveedor::{
    IngresoProveedor, IngresoProveedorGuardado, IngresoProveedorId,
};
use limen_dominio::medio::{Medio, Placa};
use limen_dominio::movimiento::Marca;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::tipo_ingreso::TipoIngreso;
use limen_dominio::visitante::Visitante;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use crate::error::dato_corrupto;

pub const TABLA_CONTRATISTA: &str = "contratista";
pub const TABLA_EMPRESA: &str = "empresa";
pub const TABLA_GAFETE: &str = "gafete";
pub const TABLA_PRESTAMO_GAFETE: &str = "prestamo_gafete";
pub const TABLA_PRESENCIA: &str = "presencia";
pub const TABLA_INGRESO_CONTRATISTA: &str = "ingreso_contratista";
pub const TABLA_EMPRESA_PROVEEDORA: &str = "empresa_proveedora";
pub const TABLA_INGRESO_PROVEEDOR: &str = "ingreso_proveedor";
pub const TABLA_RELOJ: &str = "reloj";
pub const TABLA_AUDITORIA: &str = "auditoria";

// --- IDs ---
//
// Las entidades con identidad propia usan su UUID como clave. Lo que debe
// ser único por naturaleza (un gafete, un préstamo, la presencia de una
// persona) usa una clave natural: así la propia base impide el duplicado.

pub fn id_registro(tabla: &str, uuid: Uuid) -> RecordId {
    RecordId::new(tabla, RecordIdKey::Uuid(uuid.into()))
}

fn id_natural(tabla: &str, clave: String) -> RecordId {
    RecordId::new(tabla, RecordIdKey::String(clave))
}

pub fn id_contratista(id: ContratistaId) -> RecordId {
    id_registro(TABLA_CONTRATISTA, id.uuid())
}

pub fn id_empresa(id: EmpresaId) -> RecordId {
    id_registro(TABLA_EMPRESA, id.uuid())
}

pub fn id_ingreso(id: IngresoId) -> RecordId {
    id_registro(TABLA_INGRESO_CONTRATISTA, id.uuid())
}

pub fn id_empresa_proveedora(id: EmpresaProveedoraId) -> RecordId {
    id_registro(TABLA_EMPRESA_PROVEEDORA, id.uuid())
}

pub fn id_ingreso_proveedor(id: IngresoProveedorId) -> RecordId {
    id_registro(TABLA_INGRESO_PROVEEDOR, id.uuid())
}

/// Clave natural de un gafete: `TIPO-NÚMERO` (por ejemplo `CONTRATISTA-25`).
pub fn clave_gafete(tipo: TipoGafete, numero: NumeroGafete) -> String {
    format!("{tipo}-{numero}")
}

pub fn id_gafete(tipo: TipoGafete, numero: NumeroGafete) -> RecordId {
    id_natural(TABLA_GAFETE, clave_gafete(tipo, numero))
}

pub fn id_prestamo(tipo: TipoGafete, numero: NumeroGafete) -> RecordId {
    id_natural(TABLA_PRESTAMO_GAFETE, clave_gafete(tipo, numero))
}

pub fn id_presencia(cedula: &Cedula) -> RecordId {
    id_natural(TABLA_PRESENCIA, cedula.as_str().to_owned())
}

/// Un solo registro guarda la hora del último movimiento del equipo.
pub fn id_reloj() -> RecordId {
    id_natural(TABLA_RELOJ, "ultimo".to_owned())
}

/// El UUID de un ID de registro, comprobando que sea de la tabla esperada.
fn uuid_de(id: &RecordId, tabla: &str) -> Result<Uuid, ErrorPersistencia> {
    match &id.key {
        RecordIdKey::Uuid(uuid) if id.table.as_str() == tabla => Ok((*uuid).into()),
        _ => Err(dato_corrupto(
            tabla,
            "el ID del registro no es un UUID de la tabla",
        )),
    }
}

fn numero_de(valor: i64, tabla: &str) -> Result<NumeroGafete, ErrorPersistencia> {
    u32::try_from(valor)
        .ok()
        .and_then(|numero| NumeroGafete::nuevo(numero).ok())
        .ok_or_else(|| dato_corrupto(tabla, format!("número de gafete inválido: {valor}")))
}

/// El medio guardado: sin placa es a pie.
fn medio_de(placa: Option<String>, tabla: &str) -> Result<Medio, ErrorPersistencia> {
    placa.map_or(Ok(Medio::APie), |placa| {
        Placa::nueva(&placa)
            .map(Medio::Vehiculo)
            .map_err(|e| dato_corrupto(tabla, e))
    })
}

/// La salida guardada: hora y operador van juntos o no van.
fn salida_de(
    en: Option<DateTime<Utc>>,
    operador: Option<Uuid>,
    tabla: &str,
) -> Result<Option<Marca>, ErrorPersistencia> {
    match (en, operador) {
        (Some(en), Some(operador)) => Ok(Some(Marca {
            en,
            operador: OperadorId::desde_uuid(operador),
        })),
        (None, None) => Ok(None),
        _ => Err(dato_corrupto(
            tabla,
            "salida a medias: falta la hora o el operador",
        )),
    }
}

// --- Contratista ---

/// Los campos que se guardan de un contratista (sin el ID, que es la clave).
#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct ContratistaDatos {
    pub cedula: String,
    pub nombre: String,
    pub empresa: RecordId,
    pub tipo_ingreso: String,
    pub fecha_vencimiento_praind: NaiveDate,
    pub tiene_acceso: bool,
}

/// Un contratista leído de la base: su ID y sus campos.
#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct ContratistaLeido {
    pub id: RecordId,
    pub cedula: String,
    pub nombre: String,
    pub empresa: RecordId,
    pub tipo_ingreso: String,
    pub fecha_vencimiento_praind: NaiveDate,
    pub tiene_acceso: bool,
}

impl From<&Contratista> for ContratistaDatos {
    fn from(contratista: &Contratista) -> Self {
        Self {
            cedula: contratista.cedula().as_str().to_owned(),
            nombre: contratista.nombre().as_str().to_owned(),
            empresa: id_empresa(contratista.empresa()),
            tipo_ingreso: contratista.tipo_ingreso().codigo().to_owned(),
            fecha_vencimiento_praind: contratista.fecha_vencimiento_praind(),
            tiene_acceso: contratista.tiene_acceso(),
        }
    }
}

impl TryFrom<ContratistaLeido> for Contratista {
    type Error = ErrorPersistencia;

    /// Reconstruye la entidad. Cédula y nombre vuelven a pasar por sus
    /// objetos de valor: un dato que no los cumple es corrupción.
    fn try_from(leido: ContratistaLeido) -> Result<Self, ErrorPersistencia> {
        let corrupto = |detalle: String| dato_corrupto(TABLA_CONTRATISTA, detalle);
        Ok(Self::restaurar(ContratistaGuardado {
            id: ContratistaId::desde_uuid(uuid_de(&leido.id, TABLA_CONTRATISTA)?),
            cedula: Cedula::normalizar(&leido.cedula).map_err(|e| corrupto(e.to_string()))?,
            nombre: NombrePersona::nuevo(&leido.nombre).map_err(|e| corrupto(e.to_string()))?,
            empresa: EmpresaId::desde_uuid(uuid_de(&leido.empresa, TABLA_EMPRESA)?),
            tipo_ingreso: TipoIngreso::desde_codigo(&leido.tipo_ingreso)
                .map_err(|e| corrupto(e.to_string()))?,
            fecha_vencimiento_praind: leido.fecha_vencimiento_praind,
            tiene_acceso: leido.tiene_acceso,
        }))
    }
}

// --- Empresa ---

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct EmpresaDatos {
    pub nombre: String,
}

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct EmpresaLeida {
    pub id: RecordId,
    pub nombre: String,
}

impl From<&Empresa> for EmpresaDatos {
    fn from(empresa: &Empresa) -> Self {
        Self {
            nombre: empresa.nombre().as_str().to_owned(),
        }
    }
}

impl TryFrom<EmpresaLeida> for Empresa {
    type Error = ErrorPersistencia;

    fn try_from(leida: EmpresaLeida) -> Result<Self, ErrorPersistencia> {
        let nombre =
            NombreEmpresa::nuevo(&leida.nombre).map_err(|e| dato_corrupto(TABLA_EMPRESA, e))?;
        Ok(Self::restaurar(
            EmpresaId::desde_uuid(uuid_de(&leida.id, TABLA_EMPRESA)?),
            nombre,
        ))
    }
}

// --- Gafete ---

/// Un gafete tal como se guarda y se lee. La clave se arma con tipo y
/// número, que también se guardan como campos para poder consultarlos.
#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct GafeteDatos {
    pub tipo: String,
    pub numero: i64,
    pub estado: String,
    pub deudor_contratista: Option<RecordId>,
    pub deudor_cedula: Option<String>,
}

impl From<&Gafete> for GafeteDatos {
    fn from(gafete: &Gafete) -> Self {
        let (deudor_contratista, deudor_cedula) = match gafete.deudor() {
            Some(Deudor::Contratista(id)) => (Some(id_contratista(*id)), None),
            Some(Deudor::Persona(cedula)) => (None, Some(cedula.as_str().to_owned())),
            None => (None, None),
        };
        Self {
            tipo: gafete.tipo().codigo().to_owned(),
            numero: i64::from(gafete.numero().valor()),
            estado: gafete.estado().codigo().to_owned(),
            deudor_contratista,
            deudor_cedula,
        }
    }
}

impl TryFrom<GafeteDatos> for Gafete {
    type Error = ErrorPersistencia;

    fn try_from(datos: GafeteDatos) -> Result<Self, ErrorPersistencia> {
        let corrupto = |detalle: String| dato_corrupto(TABLA_GAFETE, detalle);
        let tipo = TipoGafete::desde_codigo(&datos.tipo)
            .ok_or_else(|| corrupto(format!("tipo desconocido: {}", datos.tipo)))?;
        let estado = EstadoGafete::desde_codigo(&datos.estado)
            .ok_or_else(|| corrupto(format!("estado desconocido: {}", datos.estado)))?;
        let deudor = match (datos.deudor_contratista, datos.deudor_cedula) {
            (Some(contratista), None) => Some(Deudor::Contratista(ContratistaId::desde_uuid(
                uuid_de(&contratista, TABLA_CONTRATISTA)?,
            ))),
            (None, Some(cedula)) => Some(Deudor::Persona(
                Cedula::normalizar(&cedula).map_err(|e| corrupto(e.to_string()))?,
            )),
            (None, None) => None,
            (Some(_), Some(_)) => return Err(corrupto("el gafete tiene dos deudores".to_owned())),
        };
        Ok(Self::restaurar(
            tipo,
            numero_de(datos.numero, TABLA_GAFETE)?,
            estado,
            deudor,
        ))
    }
}

/// Un préstamo de gafete: desde cuándo está prestado.
#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct PrestamoDatos {
    pub desde: DateTime<Utc>,
}

// --- Presencia ---

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct PresenciaDatos {
    pub via: String,
    pub desde: DateTime<Utc>,
}

// --- Ingreso de contratista ---

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct IngresoDatos {
    pub contratista: RecordId,
    pub cedula: String,
    pub placa: Option<String>,
    pub gafete: Option<i64>,
    pub entrada_en: DateTime<Utc>,
    pub entrada_operador: Uuid,
    pub salida_en: Option<DateTime<Utc>>,
    pub salida_operador: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct IngresoLeido {
    pub id: RecordId,
    pub contratista: RecordId,
    pub cedula: String,
    pub placa: Option<String>,
    pub gafete: Option<i64>,
    pub entrada_en: DateTime<Utc>,
    pub entrada_operador: Uuid,
    pub salida_en: Option<DateTime<Utc>>,
    pub salida_operador: Option<Uuid>,
}

impl From<&IngresoContratista> for IngresoDatos {
    fn from(ingreso: &IngresoContratista) -> Self {
        let salida = ingreso.salida();
        Self {
            contratista: id_contratista(ingreso.contratista()),
            cedula: ingreso.cedula().as_str().to_owned(),
            placa: ingreso
                .medio()
                .placa()
                .map(|placa| placa.as_str().to_owned()),
            gafete: ingreso.gafete().map(|numero| i64::from(numero.valor())),
            entrada_en: ingreso.entrada().en,
            entrada_operador: ingreso.entrada().operador.uuid(),
            salida_en: salida.map(|marca| marca.en),
            salida_operador: salida.map(|marca| marca.operador.uuid()),
        }
    }
}

impl TryFrom<IngresoLeido> for IngresoContratista {
    type Error = ErrorPersistencia;

    fn try_from(leido: IngresoLeido) -> Result<Self, ErrorPersistencia> {
        let tabla = TABLA_INGRESO_CONTRATISTA;
        let corrupto = |detalle: String| dato_corrupto(tabla, detalle);
        let medio = medio_de(leido.placa, tabla)?;
        let salida = salida_de(leido.salida_en, leido.salida_operador, tabla)?;
        Ok(Self::restaurar(IngresoGuardado {
            id: IngresoId::desde_uuid(uuid_de(&leido.id, tabla)?),
            contratista: ContratistaId::desde_uuid(uuid_de(&leido.contratista, TABLA_CONTRATISTA)?),
            cedula: Cedula::normalizar(&leido.cedula).map_err(|e| corrupto(e.to_string()))?,
            medio,
            gafete: leido.gafete.map(|n| numero_de(n, tabla)).transpose()?,
            entrada: Marca {
                en: leido.entrada_en,
                operador: OperadorId::desde_uuid(leido.entrada_operador),
            },
            salida,
        }))
    }
}

// --- Empresa proveedora ---

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct EmpresaProveedoraLeida {
    pub id: RecordId,
    pub nombre: String,
}

impl From<&EmpresaProveedora> for EmpresaDatos {
    fn from(empresa: &EmpresaProveedora) -> Self {
        Self {
            nombre: empresa.nombre().as_str().to_owned(),
        }
    }
}

impl TryFrom<EmpresaProveedoraLeida> for EmpresaProveedora {
    type Error = ErrorPersistencia;

    fn try_from(leida: EmpresaProveedoraLeida) -> Result<Self, ErrorPersistencia> {
        let tabla = TABLA_EMPRESA_PROVEEDORA;
        let nombre = NombreEmpresa::nuevo(&leida.nombre).map_err(|e| dato_corrupto(tabla, e))?;
        Ok(Self::restaurar(
            EmpresaProveedoraId::desde_uuid(uuid_de(&leida.id, tabla)?),
            nombre,
        ))
    }
}

// --- Ingreso de proveedor ---

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct IngresoProveedorDatos {
    pub cedula: String,
    pub nombre: String,
    pub empresa: RecordId,
    pub placa: Option<String>,
    pub gafete: i64,
    pub entrada_en: DateTime<Utc>,
    pub entrada_operador: Uuid,
    pub salida_en: Option<DateTime<Utc>>,
    pub salida_operador: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct IngresoProveedorLeido {
    pub id: RecordId,
    pub cedula: String,
    pub nombre: String,
    pub empresa: RecordId,
    pub placa: Option<String>,
    pub gafete: i64,
    pub entrada_en: DateTime<Utc>,
    pub entrada_operador: Uuid,
    pub salida_en: Option<DateTime<Utc>>,
    pub salida_operador: Option<Uuid>,
}

impl From<&IngresoProveedor> for IngresoProveedorDatos {
    fn from(ingreso: &IngresoProveedor) -> Self {
        let salida = ingreso.salida();
        Self {
            cedula: ingreso.cedula().as_str().to_owned(),
            nombre: ingreso.visitante().nombre().as_str().to_owned(),
            empresa: id_empresa_proveedora(ingreso.empresa()),
            placa: ingreso
                .medio()
                .placa()
                .map(|placa| placa.as_str().to_owned()),
            gafete: i64::from(ingreso.gafete().valor()),
            entrada_en: ingreso.entrada().en,
            entrada_operador: ingreso.entrada().operador.uuid(),
            salida_en: salida.map(|marca| marca.en),
            salida_operador: salida.map(|marca| marca.operador.uuid()),
        }
    }
}

impl TryFrom<IngresoProveedorLeido> for IngresoProveedor {
    type Error = ErrorPersistencia;

    fn try_from(leido: IngresoProveedorLeido) -> Result<Self, ErrorPersistencia> {
        let tabla = TABLA_INGRESO_PROVEEDOR;
        let corrupto = |detalle: String| dato_corrupto(tabla, detalle);
        let visitante = Visitante::restaurar(
            Cedula::normalizar(&leido.cedula).map_err(|e| corrupto(e.to_string()))?,
            NombrePersona::nuevo(&leido.nombre).map_err(|e| corrupto(e.to_string()))?,
        );
        Ok(Self::restaurar(IngresoProveedorGuardado {
            id: IngresoProveedorId::desde_uuid(uuid_de(&leido.id, tabla)?),
            visitante,
            empresa: EmpresaProveedoraId::desde_uuid(uuid_de(
                &leido.empresa,
                TABLA_EMPRESA_PROVEEDORA,
            )?),
            medio: medio_de(leido.placa, tabla)?,
            gafete: numero_de(leido.gafete, tabla)?,
            entrada: Marca {
                en: leido.entrada_en,
                operador: OperadorId::desde_uuid(leido.entrada_operador),
            },
            salida: salida_de(leido.salida_en, leido.salida_operador, tabla)?,
        }))
    }
}

// --- Reloj ---

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct RelojDatos {
    pub en: DateTime<Utc>,
}

// --- Auditoría ---

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct CambioRegistro {
    pub campo: String,
    pub antes: String,
    pub despues: String,
}

/// Una entrada de auditoría tal como se guarda (y se lee).
#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
pub struct AuditoriaRegistro {
    pub entidad: String,
    pub registro: String,
    pub accion: String,
    pub cambios: Vec<CambioRegistro>,
    pub operador: Uuid,
    pub en: DateTime<Utc>,
}

impl From<&EntradaAuditoria> for AuditoriaRegistro {
    fn from(entrada: &EntradaAuditoria) -> Self {
        Self {
            entidad: entrada.registro.entidad().to_owned(),
            registro: entrada.registro.clave(),
            accion: entrada.accion.codigo().to_owned(),
            cambios: entrada
                .cambios
                .iter()
                .map(|cambio| CambioRegistro {
                    campo: cambio.campo.to_owned(),
                    antes: cambio.antes.clone(),
                    despues: cambio.despues.clone(),
                })
                .collect(),
            operador: entrada.operador.uuid(),
            en: entrada.en,
        }
    }
}
