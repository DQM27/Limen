//! Casos de uso: un struct por operación, con sólo los puertos que
//! necesita. Siempre en el mismo orden:
//!
//! 1. validar el formato con el dominio (sin tocar la base);
//! 2. leer los hechos que el dominio necesita;
//! 3. pedirle al dominio que decida;
//! 4. anotar las escrituras y la auditoría;
//! 5. confirmar la Unit of Work.
//!
//! Si aparece un `if` que decide algo de negocio, va al dominio.

pub mod contratistas;
pub mod empresas;
pub mod gafetes;
pub mod ingresos;
pub mod proveedores;
mod veto;
