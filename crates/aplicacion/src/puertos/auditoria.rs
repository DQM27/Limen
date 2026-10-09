use chrono::{DateTime, Utc};
use limen_dominio::auditoria::CambioCampo;
use uuid::Uuid;

use crate::sesion::Sesion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntidadAuditada {
    Contratista,
    Empresa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionAuditada {
    Alta,
    Edicion,
}

/// Un registro de auditoría: qué cambió (lo decide el dominio), quién y
/// cuándo (lo agrega el caso de uso).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntradaAuditoria {
    pub entidad: EntidadAuditada,
    pub id: Uuid,
    pub accion: AccionAuditada,
    pub cambios: Vec<CambioCampo>,
    pub operador: crate::sesion::OperadorId,
    pub en: DateTime<Utc>,
}

impl EntradaAuditoria {
    pub fn nueva(
        entidad: EntidadAuditada,
        id: Uuid,
        accion: AccionAuditada,
        cambios: Vec<CambioCampo>,
        sesion: &Sesion,
        en: DateTime<Utc>,
    ) -> Self {
        Self {
            entidad,
            id,
            accion,
            cambios,
            operador: sesion.operador(),
            en,
        }
    }
}

/// Anota entradas de auditoría dentro de una Unit of Work: se guardan
/// junto con el cambio que describen, o no se guarda ninguno de los dos.
pub trait RegistroAuditoria: Send + Sync {
    fn anotar(&mut self, entrada: EntradaAuditoria);
}
