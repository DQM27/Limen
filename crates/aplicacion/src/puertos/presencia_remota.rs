use std::future::Future;

use limen_dominio::presencia::{AdentroEnOtraUnidad, Identidad};

/// Pregunta a la nube si una persona ya está adentro en otra unidad (regla A9).
///
/// La respuesta es `None` si la persona no está adentro, si el equipo no tiene
/// nube o si la nube no contestó a tiempo: el acceso nunca se bloquea por la
/// red, y el choque, si lo hay, se marca al sincronizar. Quien implementa el
/// puerto no decide nada del negocio; sólo trae lo que la nube sabe.
pub trait PresenciaRemota: Send + Sync {
    fn consultar(
        &self,
        identidad: &Identidad,
    ) -> impl Future<Output = Option<AdentroEnOtraUnidad>> + Send;
}
