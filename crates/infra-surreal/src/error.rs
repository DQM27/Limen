//! Traducción de los errores de `SurrealDB` a los del puerto.
//!
//! Los errores de `SurrealDB` no salen de este crate. Un choque se reconoce
//! por el nombre del índice único o de la tabla con clave natural (ambos
//! definidos en el esquema) y se convierte en
//! `ErrorPersistencia::Conflicto`; todo lo demás es una falla técnica.

use limen_aplicacion::puertos::{ErrorPersistencia, Restriccion};

/// Marcas en el mensaje de error y la restricción que representan.
///
/// - Índices únicos: el mensaje nombra el índice.
/// - Claves naturales: crear un registro que ya existe falla con
///   "Database record \`tabla:clave\` already exists"; se busca la tabla
///   con la comilla invertida delante, para que `gafete:` no confunda con
///   `prestamo_gafete:`.
const MARCAS_DE_CONFLICTO: [(&str, Restriccion); 6] = [
    ("contratista_cedula_unica", Restriccion::CedulaContratista),
    (
        "empresa_proveedora_nombre_unico",
        Restriccion::NombreEmpresaProveedora,
    ),
    ("empresa_nombre_unico", Restriccion::NombreEmpresa),
    ("`prestamo_gafete:", Restriccion::GafetePrestado),
    ("`gafete:", Restriccion::NumeroGafete),
    ("`presencia:", Restriccion::PresenciaPersona),
];

pub fn tecnica(error: surrealdb::Error) -> ErrorPersistencia {
    ErrorPersistencia::Tecnica(error.to_string())
}

/// Error al confirmar: puede ser un choque con una restricción.
pub fn al_confirmar(error: &surrealdb::Error) -> ErrorPersistencia {
    let mensaje = error.to_string();
    MARCAS_DE_CONFLICTO
        .iter()
        .find(|(marca, _)| mensaje.contains(marca))
        .map_or_else(
            || ErrorPersistencia::Tecnica(mensaje.clone()),
            |(_, restriccion)| ErrorPersistencia::Conflicto(*restriccion),
        )
}

/// Un dato guardado que ya no cumple su formato: no debería pasar nunca
/// (sólo se guarda lo que validó el dominio), así que es técnico.
pub fn dato_corrupto(tabla: &str, detalle: impl std::fmt::Display) -> ErrorPersistencia {
    ErrorPersistencia::Tecnica(format!("dato guardado inválido en `{tabla}`: {detalle}"))
}
