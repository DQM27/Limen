//! Capa de aplicación de Limen: casos de uso y puertos.
//!
//! - Los **casos de uso** orquestan una operación: leen datos por los
//!   puertos, le piden al dominio que decida, anotan las escrituras y las
//!   confirman. No contienen reglas de negocio.
//! - Los **puertos** son los traits que esta capa necesita del exterior
//!   (base de datos, reloj, IDs). Se definen acá y los implementan los
//!   crates `infra-*`: así las dependencias apuntan hacia adentro.
//!
//! Ver `docs/arquitectura.md`, secciones 5 a 7.

pub mod casos_de_uso;
pub mod errores;
pub mod puertos;
pub mod sesion;
