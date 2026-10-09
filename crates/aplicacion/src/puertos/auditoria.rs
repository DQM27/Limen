use std::fmt;

use chrono::{DateTime, Utc};
use limen_dominio::auditoria::CambioCampo;
use limen_dominio::contratista::ContratistaId;
use limen_dominio::empresa::EmpresaId;
use limen_dominio::empresa_proveedora::EmpresaProveedoraId;
use limen_dominio::gafete::{NumeroGafete, TipoGafete};
use limen_dominio::operador::OperadorId;
use uuid::Uuid;

use crate::sesion::Sesion;

/// Qué registro cambió. Cada entidad con su propia identidad: un gafete no
/// tiene UUID, se identifica por su tipo y su número.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistroAuditado {
    Contratista(ContratistaId),
    Empresa(EmpresaId),
    EmpresaProveedora(EmpresaProveedoraId),
    Gafete(TipoGafete, NumeroGafete),
}

impl RegistroAuditado {
    /// Nombre estable de la entidad.
    pub const fn entidad(self) -> &'static str {
        match self {
            Self::Contratista(_) => "contratista",
            Self::Empresa(_) => "empresa",
            Self::EmpresaProveedora(_) => "empresa_proveedora",
            Self::Gafete(..) => "gafete",
        }
    }

    /// Clave estable del registro dentro de su entidad.
    pub fn clave(self) -> String {
        match self {
            Self::Contratista(id) => id.to_string(),
            Self::Empresa(id) => id.to_string(),
            Self::EmpresaProveedora(id) => id.to_string(),
            Self::Gafete(tipo, numero) => format!("{tipo}-{numero}"),
        }
    }
}

impl fmt::Display for RegistroAuditado {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.entidad(), self.clave())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionAuditada {
    Alta,
    Edicion,
}

impl AccionAuditada {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Alta => "alta",
            Self::Edicion => "edicion",
        }
    }
}

/// Un registro de auditoría: qué cambió (lo decide el dominio), quién y
/// cuándo (lo agrega el caso de uso).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntradaAuditoria {
    /// ID propio de la entrada (UUID v7 en producción). Es ordenable y es lo
    /// que fija el orden del historial: dos cambios en el mismo instante
    /// tienen la misma hora, pero nunca el mismo ID.
    pub id_entrada: Uuid,
    pub registro: RegistroAuditado,
    pub accion: AccionAuditada,
    pub cambios: Vec<CambioCampo>,
    pub operador: OperadorId,
    pub en: DateTime<Utc>,
}

impl EntradaAuditoria {
    pub fn nueva(
        id_entrada: Uuid,
        registro: RegistroAuditado,
        accion: AccionAuditada,
        cambios: Vec<CambioCampo>,
        sesion: &Sesion,
        en: DateTime<Utc>,
    ) -> Self {
        Self {
            id_entrada,
            registro,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_registro_tiene_entidad_y_clave_estables() {
        let gafete =
            RegistroAuditado::Gafete(TipoGafete::Contratista, NumeroGafete::nuevo(25).unwrap());
        assert_eq!(gafete.entidad(), "gafete");
        assert_eq!(gafete.clave(), "CONTRATISTA-25");
        let contratista =
            RegistroAuditado::Contratista(ContratistaId::desde_uuid(Uuid::from_u128(1)));
        assert_eq!(contratista.entidad(), "contratista");
        assert_eq!(contratista.clave(), "00000000-0000-0000-0000-000000000001");
    }
}
