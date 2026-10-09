//! Presencia: quién está adentro y por qué vía (reglas E1 y A7).
//!
//! Una persona sólo puede estar adentro por una vía a la vez: si ya entró
//! como proveedor, no entra también como contratista, y no puede tener dos
//! ingresos abiertos por la misma vía. Las vías que identifican a la
//! persona por cédula se comparan por la cédula normalizada.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Via {
    Contratista,
    Proveedor,
    Correo,
}

impl Via {
    pub const TODAS: [Self; 3] = [Self::Contratista, Self::Proveedor, Self::Correo];

    /// Código estable para guardar la vía.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Contratista => "CONTRATISTA",
            Self::Proveedor => "PROVEEDOR",
            Self::Correo => "CORREO",
        }
    }

    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|via| via.codigo() == codigo)
    }

    const fn descripcion(self) -> &'static str {
        match self {
            Self::Contratista => "contratista",
            Self::Proveedor => "proveedor",
            Self::Correo => "ingreso por correo",
        }
    }
}

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.descripcion())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Esta persona ya está adentro como {0}: registre primero su salida")]
pub struct YaEstaAdentro(pub Via);

impl YaEstaAdentro {
    pub const fn codigo(self) -> &'static str {
        "ya_esta_adentro"
    }
}

/// `Ok` si la persona no está adentro por ninguna vía.
pub const fn verificar_afuera(adentro_por: Option<Via>) -> Result<(), YaEstaAdentro> {
    match adentro_por {
        Some(via) => Err(YaEstaAdentro(via)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn afuera_puede_entrar() {
        assert_eq!(verificar_afuera(None), Ok(()));
    }

    #[test]
    fn adentro_por_cualquier_via_bloquea_e_informa_cual() {
        for via in Via::TODAS {
            assert_eq!(verificar_afuera(Some(via)), Err(YaEstaAdentro(via)));
        }
        assert_eq!(
            YaEstaAdentro(Via::Proveedor).to_string(),
            "Esta persona ya está adentro como proveedor: registre primero su salida"
        );
    }

    #[test]
    fn los_codigos_van_y_vuelven() {
        for via in Via::TODAS {
            assert_eq!(Via::desde_codigo(via.codigo()), Some(via));
        }
    }
}
