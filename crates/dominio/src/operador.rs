//! Operador: quien registra cada movimiento y cada cambio. Hay un solo rol
//! (regla L1); el dominio sólo necesita saber quién fue.

use std::fmt;

use uuid::Uuid;

/// Identificador global de un operador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperadorId(Uuid);

impl OperadorId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for OperadorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
