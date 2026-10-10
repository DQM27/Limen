//! Hechos: cada entrada y salida queda registrada para siempre (ver
//! [`limen_dominio::hecho`]).

use limen_dominio::hecho::Hecho;

/// Anota hechos dentro de una Unit of Work: se guardan junto con el
/// movimiento que describen, o no se guarda ninguno de los dos. Sólo se
/// agregan: no hay forma de editar ni de borrar un hecho.
pub trait RegistroHechos: Send + Sync {
    fn anotar(&mut self, hecho: Hecho);
}
