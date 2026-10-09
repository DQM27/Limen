//! Modelos de persistencia: cómo se guarda cada entidad en `SurrealDB`, y
//! su conversión hacia y desde el dominio.
//!
//! Son structs distintos de las entidades a propósito: el dominio no conoce
//! `SurrealDB`, y la forma de guardar puede cambiar sin tocar las reglas.

use chrono::{DateTime, NaiveDate, Utc};
use limen_aplicacion::puertos::{
    AccionAuditada, EntidadAuditada, EntradaAuditoria, ErrorPersistencia,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaGuardado, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::nombre::NombrePersona;
use limen_dominio::tipo_ingreso::TipoIngreso;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use crate::error::dato_corrupto;

pub const TABLA_CONTRATISTA: &str = "contratista";
pub const TABLA_EMPRESA: &str = "empresa";
pub const TABLA_PRESENCIA: &str = "presencia";
pub const TABLA_AUDITORIA: &str = "auditoria";

// --- IDs: la clave de cada registro es el UUID de la entidad ---

pub fn id_registro(tabla: &str, uuid: Uuid) -> RecordId {
    RecordId::new(tabla, RecordIdKey::Uuid(uuid.into()))
}

pub fn id_contratista(id: ContratistaId) -> RecordId {
    id_registro(TABLA_CONTRATISTA, id.uuid())
}

pub fn id_empresa(id: EmpresaId) -> RecordId {
    id_registro(TABLA_EMPRESA, id.uuid())
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
    pub registro: Uuid,
    pub accion: String,
    pub cambios: Vec<CambioRegistro>,
    pub operador: Uuid,
    pub en: DateTime<Utc>,
}

impl From<&EntradaAuditoria> for AuditoriaRegistro {
    fn from(entrada: &EntradaAuditoria) -> Self {
        Self {
            entidad: match entrada.entidad {
                EntidadAuditada::Contratista => "contratista",
                EntidadAuditada::Empresa => "empresa",
            }
            .to_owned(),
            registro: entrada.id,
            accion: match entrada.accion {
                AccionAuditada::Alta => "alta",
                AccionAuditada::Edicion => "edicion",
            }
            .to_owned(),
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
