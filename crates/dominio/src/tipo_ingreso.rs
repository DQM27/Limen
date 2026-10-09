//! Tipo de ingreso de un contratista: decide qué se le exige al entrar.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TipoIngreso {
    /// Contratista externo: entra con PRAIND y gafete.
    Praind,
    /// Contratista que trabaja de planta en la instalación: entra con
    /// PRAIND, sin gafete.
    InHouse,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("tipo de ingreso desconocido: {0:?}")]
pub struct TipoIngresoDesconocido(pub String);

impl TipoIngreso {
    pub const TODOS: [Self; 2] = [Self::Praind, Self::InHouse];

    /// Código estable para guardar e intercambiar el tipo. No cambia aunque
    /// cambie cómo se muestra.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Praind => "PRAIND",
            Self::InHouse => "IN_HOUSE",
        }
    }

    /// Inverso de [`TipoIngreso::codigo`].
    pub fn desde_codigo(codigo: &str) -> Result<Self, TipoIngresoDesconocido> {
        Self::TODOS
            .into_iter()
            .find(|tipo| tipo.codigo() == codigo)
            .ok_or_else(|| TipoIngresoDesconocido(codigo.to_string()))
    }

    /// Sólo PRAIND lleva gafete; IN HOUSE no.
    pub const fn requiere_gafete(self) -> bool {
        matches!(self, Self::Praind)
    }
}

impl fmt::Display for TipoIngreso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.codigo())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_codigo_va_y_vuelve() {
        for tipo in TipoIngreso::TODOS {
            assert_eq!(TipoIngreso::desde_codigo(tipo.codigo()), Ok(tipo));
        }
    }

    #[test]
    fn rechaza_los_tipos_retirados() {
        for retirado in ["SWAT", "POR_CORREO", "praind", ""] {
            assert!(TipoIngreso::desde_codigo(retirado).is_err(), "{retirado}");
        }
    }

    #[test]
    fn solo_praind_lleva_gafete() {
        assert!(TipoIngreso::Praind.requiere_gafete());
        assert!(!TipoIngreso::InHouse.requiere_gafete());
    }
}
