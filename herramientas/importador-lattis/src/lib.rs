//! Importador de Lattis: herramienta de **desarrollo** que carga en Limen un
//! volcado SQL de producción de Lattis, adaptado a las reglas de Limen.
//!
//! Sirve para tener una base de pruebas con datos realistas y es el
//! ensayo de la migración real (paso 9 de la hoja de ruta). Los datos reales
//! nunca viven en este repositorio (es público): se leen de una carpeta
//! ignorada por git, y el programa sólo imprime conteos y códigos de
//! rechazo, nunca datos de personas. Las pruebas usan datos inventados.
//!
//! - [`lectura`]: lee los `INSERT ... VALUES` del volcado.
//! - [`traduccion`]: de una fila de Lattis a una entidad del dominio, con las
//!   reglas de adaptación.
//! - [`importacion`]: carga por los puertos de `aplicacion`.

pub mod importacion;
pub mod lectura;
pub mod traduccion;

pub use importacion::{Datos, ErrorImportacion, Rechazo, Resumen, ResumenTabla, importar};
