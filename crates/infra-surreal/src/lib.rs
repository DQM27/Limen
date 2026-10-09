//! Adaptador de `SurrealDB` embebido para los puertos de Limen.
//!
//! - Motores: `kv-surrealkv` en disco (equipos) y `kv-mem` (pruebas). Ambos
//!   escritos en Rust; `RocksDB` está prohibido.
//! - Sin ORM: las consultas están escritas a la vista, siempre con
//!   parámetros enlazados (`$cedula`), nunca concatenando texto.
//! - Los modelos de persistencia (`registros`) son distintos de las
//!   entidades del dominio: el dominio no sabe que existe `SurrealDB`.
//! - La Unit of Work anota las escrituras y las confirma en una sola
//!   transacción (`BEGIN … COMMIT`).
//!
//! Ver `docs/arquitectura.md`, secciones 7 y 8.

mod almacen;
mod error;
mod registros;
mod repositorios;
mod unidad_de_trabajo;

pub use almacen::AlmacenSurreal;
pub use registros::{AuditoriaRegistro, CambioRegistro};
pub use repositorios::{
    AuditoriaSurreal, ContratistasSurreal, EmpresasProveedorasSurreal, EmpresasSurreal,
    GafetesSurreal, IngresosCorreoSurreal, IngresosProveedorSurreal, IngresosSurreal,
    PresenciasSurreal, RelojSurreal,
};
pub use unidad_de_trabajo::UowSurreal;
