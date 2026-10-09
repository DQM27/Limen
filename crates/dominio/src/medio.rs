//! Medio de ingreso: a pie o en vehículo (regla E2).
//!
//! En vehículo la placa es obligatoria. A pie no hay placa: lo que se haya
//! escrito (el operador pudo escribirla antes de cambiar de medio) se
//! descarta. Por eso la placa vive dentro de la variante `Vehiculo` y no
//! puede existir un ingreso a pie con placa.

use std::fmt;

/// Largo máximo de una placa.
pub const LARGO_MAXIMO_PLACA: usize = 20;

/// Placa ya normalizada: en mayúsculas, sin espacios de más; sólo letras,
/// números, espacios y guiones.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Placa(String);

impl Placa {
    pub fn nueva(texto: &str) -> Result<Self, ErrorMedio> {
        let limpia = texto
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_uppercase();
        if limpia.is_empty() {
            return Err(ErrorMedio::PlacaRequerida);
        }
        let valida = limpia.chars().count() <= LARGO_MAXIMO_PLACA
            && limpia
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-');
        if !valida {
            return Err(ErrorMedio::PlacaInvalida);
        }
        Ok(Self(limpia))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Placa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lo que elige el operador en el formulario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoMedio {
    APie,
    Vehiculo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Medio {
    APie,
    Vehiculo(Placa),
}

impl Medio {
    /// Arma el medio a partir del formulario aplicando la regla E2.
    pub fn desde_formulario(tipo: TipoMedio, placa: Option<&str>) -> Result<Self, ErrorMedio> {
        match tipo {
            TipoMedio::APie => Ok(Self::APie),
            TipoMedio::Vehiculo => Ok(Self::Vehiculo(Placa::nueva(placa.unwrap_or_default())?)),
        }
    }

    pub const fn placa(&self) -> Option<&Placa> {
        match self {
            Self::APie => None,
            Self::Vehiculo(placa) => Some(placa),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorMedio {
    #[error("La placa es obligatoria cuando el ingreso es en vehículo")]
    PlacaRequerida,
    #[error("La placa sólo admite letras, números, espacios y guiones, hasta 20 caracteres")]
    PlacaInvalida,
}

impl ErrorMedio {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::PlacaRequerida => "placa_requerida",
            Self::PlacaInvalida => "placa_invalida",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_vehiculo_la_placa_se_normaliza() {
        let medio = Medio::desde_formulario(TipoMedio::Vehiculo, Some("  abc  123 ")).unwrap();
        assert_eq!(medio.placa().map(Placa::as_str), Some("ABC 123"));
    }

    #[test]
    fn en_vehiculo_sin_placa_se_rechaza() {
        for vacia in [None, Some(""), Some("   ")] {
            assert_eq!(
                Medio::desde_formulario(TipoMedio::Vehiculo, vacia),
                Err(ErrorMedio::PlacaRequerida),
                "{vacia:?}"
            );
        }
    }

    #[test]
    fn la_placa_rechaza_simbolos_y_largo_excesivo() {
        assert_eq!(Placa::nueva("ABC@1"), Err(ErrorMedio::PlacaInvalida));
        assert_eq!(
            Placa::nueva(&"A".repeat(21)),
            Err(ErrorMedio::PlacaInvalida)
        );
        assert!(Placa::nueva("CL-123456").is_ok(), "el guion vale");
    }

    #[test]
    fn a_pie_se_descarta_lo_que_se_haya_escrito() {
        assert_eq!(
            Medio::desde_formulario(TipoMedio::APie, Some("ABC123")),
            Ok(Medio::APie)
        );
        assert_eq!(Medio::APie.placa(), None);
    }
}
