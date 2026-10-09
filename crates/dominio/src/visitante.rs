//! Quien entra sin estar en un catálogo de personas: proveedores e ingresos
//! por correo (reglas H2 e I1).
//!
//! No hay catálogo porque cada colaborador de un proveedor es distinto: en
//! cada ingreso se toman su cédula y su nombre tal como llegan, con las
//! mismas reglas que un contratista (A1, A3, A5, A6). Además, a un
//! visitante se le aplica el veto por persona (A8): si su cédula tiene el
//! acceso denegado, no entra por ninguna vía.

use crate::cedula::{Cedula, CedulaInvalida};
use crate::nombre::{NombreInvalido, NombrePersona};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorVisitante {
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    #[error("La cédula debe tener sólo números, entre 9 y 13 dígitos")]
    CedulaInvalida,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("El nombre sólo puede tener letras y espacios")]
    NombreInvalido,
}

impl ErrorVisitante {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::CedulaVacia => "cedula_vacia",
            Self::CedulaInvalida => "cedula_invalida",
            Self::NombreVacio => "nombre_vacio",
            Self::NombreInvalido => "nombre_invalido",
        }
    }
}

/// Cédula y nombre de quien entra, ya normalizados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visitante {
    cedula: Cedula,
    nombre: NombrePersona,
}

impl Visitante {
    /// Valida y normaliza lo escrito (o leído de la cédula). Sólo cédula
    /// nacional o de extranjero (A3).
    pub fn nuevo(cedula: &str, nombre: &str) -> Result<Self, ErrorVisitante> {
        let cedula = match Cedula::normalizar(cedula) {
            Ok(cedula) if cedula.es_nacional_o_de_extranjero() => cedula,
            Err(CedulaInvalida::Vacia) => return Err(ErrorVisitante::CedulaVacia),
            Ok(_) | Err(_) => return Err(ErrorVisitante::CedulaInvalida),
        };
        let nombre = NombrePersona::nuevo(nombre).map_err(|error| match error {
            NombreInvalido::Vacio => ErrorVisitante::NombreVacio,
            NombreInvalido::CaracteresNoPermitidos => ErrorVisitante::NombreInvalido,
        })?;
        Ok(Self { cedula, nombre })
    }

    /// Reconstruye un visitante guardado. Los tipos garantizan que cédula y
    /// nombre ya pasaron por su normalización.
    pub const fn restaurar(cedula: Cedula, nombre: NombrePersona) -> Self {
        Self { cedula, nombre }
    }

    pub const fn cedula(&self) -> &Cedula {
        &self.cedula
    }

    pub const fn nombre(&self) -> &NombrePersona {
        &self.nombre
    }
}

/// Veto por persona (A8): la cédula tiene el acceso denegado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Acceso denegado para esta persona")]
pub struct PersonaVetada;

impl PersonaVetada {
    /// Mismo código que el contratista sin acceso: para quien opera, es el
    /// mismo aviso.
    pub const fn codigo(self) -> &'static str {
        "sin_acceso"
    }
}

/// `Ok` si la persona no está vetada.
pub const fn verificar_veto(vetada: bool) -> Result<(), PersonaVetada> {
    if vetada { Err(PersonaVetada) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_cedula_y_nombre() {
        let visitante = Visitante::nuevo(" 1-1111-1111 ", "josé peña").unwrap();
        assert_eq!(visitante.cedula().as_str(), "111111111");
        assert_eq!(visitante.nombre().as_str(), "JOSE PEÑA");
    }

    #[test]
    fn cedula_obligatoria_y_solo_nacional_o_de_extranjero() {
        assert_eq!(
            Visitante::nuevo("  ", "ANA"),
            Err(ErrorVisitante::CedulaVacia)
        );
        assert_eq!(
            Visitante::nuevo("AB123456", "ANA"),
            Err(ErrorVisitante::CedulaInvalida),
            "un pasaporte no sirve"
        );
        assert_eq!(
            Visitante::nuevo("1234", "ANA"),
            Err(ErrorVisitante::CedulaInvalida),
            "demasiado corta"
        );
    }

    #[test]
    fn nombre_obligatorio_y_solo_letras() {
        assert_eq!(
            Visitante::nuevo("111111111", " "),
            Err(ErrorVisitante::NombreVacio)
        );
        assert_eq!(
            Visitante::nuevo("111111111", "O'BRIEN"),
            Err(ErrorVisitante::NombreInvalido)
        );
    }

    #[test]
    fn la_cedula_se_valida_antes_que_el_nombre() {
        assert_eq!(Visitante::nuevo("", ""), Err(ErrorVisitante::CedulaVacia));
    }

    #[test]
    fn el_veto_rechaza_con_el_mismo_codigo_que_sin_acceso() {
        assert_eq!(verificar_veto(false), Ok(()));
        assert_eq!(verificar_veto(true), Err(PersonaVetada));
        assert_eq!(PersonaVetada.codigo(), "sin_acceso");
    }
}
