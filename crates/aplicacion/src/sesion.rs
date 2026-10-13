//! Quién está operando y dónde. Hay un solo rol (Operador, regla L1): la
//! sesión no decide permisos, sólo identifica a quien hace cada cambio para
//! la auditoría, y en qué unidad operativa y desde qué equipo lo hace (M4),
//! para el origen de cada hecho.

pub use limen_dominio::operador::OperadorId;
pub use limen_dominio::origen::Origen;

/// Sesión abierta de un operador. En la aplicación la construyen el inicio
/// de sesión y la creación del primer usuario; `nueva` queda para las
/// pruebas y para los adaptadores que restauran una sesión ya abierta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sesion {
    operador: OperadorId,
    origen: Origen,
}

impl Sesion {
    pub const fn nueva(operador: OperadorId, origen: Origen) -> Self {
        Self { operador, origen }
    }

    pub const fn operador(&self) -> OperadorId {
        self.operador
    }

    /// La unidad operativa y el equipo donde está abierta la sesión.
    pub const fn origen(&self) -> &Origen {
        &self.origen
    }
}
