//! Puertos: lo que la aplicación necesita del exterior, como traits.
//!
//! Los métodos asíncronos devuelven `impl Future + Send` (en vez de
//! `async fn`) para que los casos de uso se puedan llamar desde cualquier
//! hilo, por ejemplo desde un comando de Tauri. Quien implementa el trait
//! puede escribir `async fn` igual.

mod auditoria;
mod persistencia;
mod reloj;

pub use auditoria::{AccionAuditada, EntidadAuditada, EntradaAuditoria, RegistroAuditoria};
pub use persistencia::{
    ConsultaPresencias, ErrorPersistencia, FabricaUnidadDeTrabajo, RepositorioContratistas,
    RepositorioEmpresas, Restriccion, UnidadDeTrabajo,
};
pub use reloj::{GeneradorIds, Reloj};
