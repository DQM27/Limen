use limen_dominio::usuario::{ClaveNueva, HashClave};

/// Cifra y verifica claves. En producción, Argon2id con sal aleatoria
/// (`infra-plataforma`). Es lento a propósito: así adivinar cuesta caro.
pub trait Claves: Send + Sync {
    /// El hash de una clave nueva. El error es una falla técnica.
    fn cifrar(&self, clave: &ClaveNueva) -> Result<HashClave, String>;

    /// Si `clave` corresponde a `hash`. Sin hash (la cédula no existe)
    /// hace el mismo trabajo y devuelve `false`: así la respuesta tarda lo
    /// mismo y no delata qué cédulas existen.
    fn verificar(&self, clave: &str, hash: Option<&HashClave>) -> bool;
}
