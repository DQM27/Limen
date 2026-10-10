use limen_dominio::usuario::{ContrasenaNueva, HashContrasena};

/// Cifra y verifica contraseñas. En producción, Argon2id con sal aleatoria
/// (`infra-plataforma`). Es lento a propósito: así adivinar cuesta caro.
pub trait Contrasenas: Send + Sync {
    /// El hash de una contraseña nueva. El error es una falla técnica.
    fn cifrar(&self, contrasena: &ContrasenaNueva) -> Result<HashContrasena, String>;

    /// Si `contrasena` corresponde a `hash`. Sin hash (la cédula no existe)
    /// hace el mismo trabajo y devuelve `false`: así la respuesta tarda lo
    /// mismo y no delata qué cédulas existen.
    fn verificar(&self, contrasena: &str, hash: Option<&HashContrasena>) -> bool;
}
