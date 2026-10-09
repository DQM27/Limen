//! Traducción de los errores de `SurrealDB` a los del puerto.
//!
//! Los errores de `SurrealDB` no salen de este crate. Un choque con un
//! índice único se reconoce por el nombre del índice (definido en el
//! esquema) y se convierte en `ErrorPersistencia::Conflicto`; todo lo demás
//! es una falla técnica.

use limen_aplicacion::puertos::{ErrorPersistencia, Restriccion};

/// Índices únicos del esquema y la restricción que representan.
const INDICES_UNICOS: [(&str, Restriccion); 2] = [
    ("contratista_cedula_unica", Restriccion::CedulaContratista),
    ("empresa_nombre_unico", Restriccion::NombreEmpresa),
];

pub fn tecnica(error: surrealdb::Error) -> ErrorPersistencia {
    ErrorPersistencia::Tecnica(error.to_string())
}

/// Error al confirmar: puede ser un choque con un índice único.
pub fn al_confirmar(error: &surrealdb::Error) -> ErrorPersistencia {
    let mensaje = error.to_string();
    INDICES_UNICOS
        .iter()
        .find(|(indice, _)| mensaje.contains(indice))
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
