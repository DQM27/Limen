//! Quién está operando. Hay un solo rol (Operador, regla L1): la sesión no
//! decide permisos, sólo identifica a quien hace cada cambio para la
//! auditoría.

pub use limen_dominio::operador::OperadorId;

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
