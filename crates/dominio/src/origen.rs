//! Origen: en qué unidad operativa y desde qué equipo se registró algo.
//!
//! Reglas de negocio (bloque M de `docs/reglas.md`):
//! - M4: cada equipo pertenece a una **unidad operativa** (el "sitio" de
//!   Lattis: Brisas, Cartago…). Los catálogos son globales; gafetes,
//!   ingresos, presencia, préstamos e historial son de la unidad.
//! - La unidad se identifica con un código corto y estable (`BRISAS`): va
//!   en las claves de lo que es de la unidad, como el gafete
//!   `BRISAS-CONTRATISTA-25`, distinto del 25 de otra unidad.
//!
//! El dominio no sabe de dónde salen la unidad y el equipo (hoy, la
//! configuración del equipo; con el bloque M, la vinculación): los recibe
//! con la sesión, como recibe la fecha de hoy.

use std::fmt;

use uuid::Uuid;

/// Cuántos caracteres puede tener el código de una unidad operativa.
pub const MAXIMO_CODIGO_UNIDAD: usize = 30;

/// Código de una unidad operativa: mayúsculas de la A a la Z, dígitos y
/// guion bajo, de 2 a [`MAXIMO_CODIGO_UNIDAD`] caracteres. Se normaliza a
/// mayúsculas y sin espacios alrededor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnidadOperativaId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorOrigen {
    #[error(
        "El código de la unidad operativa debe tener de 2 a {MAXIMO_CODIGO_UNIDAD} letras (A-Z), números o guion bajo"
    )]
    CodigoUnidadInvalido,
}

impl ErrorOrigen {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::CodigoUnidadInvalido => "codigo_unidad_invalido",
        }
    }
}

impl UnidadOperativaId {
    pub fn nueva(codigo: &str) -> Result<Self, ErrorOrigen> {
        let codigo = codigo.trim().to_ascii_uppercase();
        let largo_valido = (2..=MAXIMO_CODIGO_UNIDAD).contains(&codigo.len());
        let caracteres_validos = codigo
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if largo_valido && caracteres_validos {
            Ok(Self(codigo))
        } else {
            Err(ErrorOrigen::CodigoUnidadInvalido)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnidadOperativaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identificador de un equipo (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EquipoId(Uuid);

impl EquipoId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for EquipoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Dónde y desde qué equipo se registró algo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origen {
    pub unidad: UnidadOperativaId,
    pub equipo: EquipoId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_codigo_de_la_unidad_se_normaliza_a_mayusculas() {
        let brisas = UnidadOperativaId::nueva("  brisas ").unwrap();
        assert_eq!(brisas.as_str(), "BRISAS");
        assert_eq!(brisas.to_string(), "BRISAS");
        assert_eq!(
            UnidadOperativaId::nueva("planta_2").unwrap().as_str(),
            "PLANTA_2"
        );
    }

    #[test]
    fn el_codigo_de_la_unidad_rechaza_lo_que_no_es_letra_numero_o_guion_bajo() {
        for malo in ["", "B", "LAS BRISAS", "BRISAS-1", "BELÉN", &"X".repeat(31)] {
            assert_eq!(
                UnidadOperativaId::nueva(malo),
                Err(ErrorOrigen::CodigoUnidadInvalido),
                "{malo:?}"
            );
        }
        assert_eq!(
            ErrorOrigen::CodigoUnidadInvalido.codigo(),
            "codigo_unidad_invalido"
        );
        assert!(
            UnidadOperativaId::nueva(&"X".repeat(30)).is_ok(),
            "30 caracteres todavía valen"
        );
    }
}
