//! Quién está operando. Hay un solo rol (Operador, regla L1): la sesión no
//! decide permisos, sólo identifica a quien hace cada cambio para la
//! auditoría.

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

/// Sesión abierta de un operador. Sólo la construye el caso de uso de
/// inicio de sesión (todavía no existe); mientras tanto, `nueva`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sesion {
    operador: OperadorId,
}

impl Sesion {
    pub const fn nueva(operador: OperadorId) -> Self {
        Self { operador }
    }

    pub const fn operador(&self) -> OperadorId {
        self.operador
    }
}
