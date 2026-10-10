//! Quién está operando. Hay un solo rol (Operador, regla L1): la sesión no
//! decide permisos, sólo identifica a quien hace cada cambio para la
//! auditoría.

pub use limen_dominio::operador::OperadorId;

/// Sesión abierta de un operador. En la aplicación la construyen el inicio
/// de sesión y la creación del primer usuario; `nueva` queda para las
/// pruebas y para los adaptadores que restauran una sesión ya abierta.
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
