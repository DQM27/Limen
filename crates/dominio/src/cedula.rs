//! Identidad de una persona: la cédula en su forma única (canónica).
//!
//! La misma persona puede llegar escrita de muchas formas: `1-1234-0567`,
//! `112340567`, `01-1234-0567` (formato del TSE con cero inicial) o leída
//! por el OCR con espacios. Si cada entrada limpiara a su manera, esas
//! formas contarían como personas distintas y una regla por cédula se
//! esquivaría cambiando el formato. Toda cédula pasa por
//! [`Cedula::normalizar`].

use std::fmt;

/// Largo máximo de cualquier documento (pasaportes incluidos). Por encima
/// es un error de tipeo o basura del OCR, no un documento.
const LARGO_MAXIMO: usize = 20;

/// Cédula ya normalizada: sin espacios, guiones ni puntos, en mayúsculas y
/// sin el cero inicial del formato del TSE. Sólo se construye con
/// [`Cedula::normalizar`], así que tener una garantiza la forma única.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cedula(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoDocumento {
    /// Cédula de identidad costarricense: 9 dígitos.
    Nacional,
    /// Documento de residente extranjero: 11 o 12 dígitos.
    Dimex,
    /// Cualquier otro documento (pasaporte, o un número de otro largo).
    Otro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CedulaInvalida {
    #[error("la cédula está vacía")]
    Vacia,
    #[error("la cédula tiene caracteres que no son letras ni números")]
    CaracteresNoPermitidos,
    #[error("la cédula es demasiado larga")]
    DemasiadoLarga,
}

impl Cedula {
    /// Forma única de `texto`:
    /// 1. quita espacios, guiones y puntos (los separadores habituales);
    /// 2. pasa a mayúsculas (pasaportes con letras);
    /// 3. 10 dígitos que empiezan en `0` (formato del TSE `0X-XXXX-XXXX`)
    ///    pierden ese cero: es la misma cédula de 9 dígitos.
    ///
    /// Rechaza sólo lo vacío, lo que trae otros símbolos y lo que pasa de
    /// 20 caracteres. Qué documentos acepta cada tipo de persona lo
    /// decide su propia regla (ver [`Cedula::es_nacional_o_de_extranjero`]).
    pub fn normalizar(texto: &str) -> Result<Self, CedulaInvalida> {
        let limpia: String = texto
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '.')
            .collect::<String>()
            .to_uppercase();
        if limpia.is_empty() {
            return Err(CedulaInvalida::Vacia);
        }
        if !limpia.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(CedulaInvalida::CaracteresNoPermitidos);
        }
        if limpia.len() > LARGO_MAXIMO {
            return Err(CedulaInvalida::DemasiadoLarga);
        }
        let es_formato_tse = limpia.len() == 10 && limpia.chars().all(|c| c.is_ascii_digit());
        let sin_cero_del_tse = match limpia.strip_prefix('0') {
            Some(resto) if es_formato_tse => resto.to_owned(),
            _ => limpia,
        };
        Ok(Self(sin_cero_del_tse))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn es_numerica(&self) -> bool {
        self.0.chars().all(|c| c.is_ascii_digit())
    }

    pub fn tipo(&self) -> TipoDocumento {
        match (self.es_numerica(), self.0.len()) {
            (true, 9) => TipoDocumento::Nacional,
            (true, 11 | 12) => TipoDocumento::Dimex,
            _ => TipoDocumento::Otro,
        }
    }

    /// Contratistas y proveedores se registran sólo con cédula nacional o de
    /// extranjero: sólo números, entre 9 y 13 dígitos. El pasaporte (con
    /// letras) sólo lo admiten las visitas.
    pub fn es_nacional_o_de_extranjero(&self) -> bool {
        self.es_numerica() && (9..=13).contains(&self.0.len())
    }
}

impl fmt::Display for Cedula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Cedula {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Casos compartidos con cualquier otra implementación de la regla (por
    /// ejemplo, una función de la base de datos): todas deben dar lo mismo.
    const VECTORES: &str = include_str!("../tests/vectores_cedula.tsv");

    #[test]
    fn cumple_todos_los_vectores_compartidos() {
        let mut probados = 0;
        for linea in VECTORES.lines().filter(|l| !l.starts_with('#')) {
            let (entrada, esperada) = linea.split_once('\t').expect("entrada<TAB>canónica");
            let esperada = (esperada != "RECHAZADA").then_some(esperada);
            let obtenida = Cedula::normalizar(entrada).ok();
            assert_eq!(
                obtenida.as_ref().map(Cedula::as_str),
                esperada,
                "entrada: {entrada:?}"
            );
            probados += 1;
        }
        assert!(probados >= 10);
    }

    #[test]
    fn formatos_de_la_misma_persona_dan_la_misma_cedula() {
        let formas = [
            "112340567",
            "1-1234-0567",
            "01-1234-0567",
            " 1 1234 0567 ",
            "1.1234.0567",
        ];
        let canonicas: Vec<_> = formas
            .iter()
            .map(|forma| Cedula::normalizar(forma).unwrap())
            .collect();
        assert!(canonicas.windows(2).all(|par| par[0] == par[1]));
        assert_eq!(canonicas[0].as_str(), "112340567");
    }

    #[test]
    fn clasifica_el_tipo_de_documento() {
        let tipo = |texto: &str| Cedula::normalizar(texto).unwrap().tipo();
        assert_eq!(tipo("112340567"), TipoDocumento::Nacional);
        assert_eq!(tipo("155812345678"), TipoDocumento::Dimex);
        assert_eq!(tipo("15581234567"), TipoDocumento::Dimex);
        assert_eq!(tipo("a1234567"), TipoDocumento::Otro);
    }

    #[test]
    fn nacional_o_de_extranjero_solo_numeros_de_9_a_13_digitos() {
        let valida = |texto: &str| {
            Cedula::normalizar(texto)
                .unwrap()
                .es_nacional_o_de_extranjero()
        };
        assert!(valida("112340567"));
        assert!(valida("1234567890123"));
        assert!(valida("0112340567"), "el cero del TSE se quita y quedan 9");
        assert!(!valida("12345678"));
        assert!(!valida("12345678901234"));
        assert!(!valida("A1234567"));
    }

    #[test]
    fn rechaza_vacia_simbolos_y_largo_excesivo() {
        assert_eq!(Cedula::normalizar("  - . "), Err(CedulaInvalida::Vacia));
        assert_eq!(
            Cedula::normalizar("1234/5678"),
            Err(CedulaInvalida::CaracteresNoPermitidos)
        );
        assert_eq!(
            Cedula::normalizar("12345ñ"),
            Err(CedulaInvalida::CaracteresNoPermitidos)
        );
        assert_eq!(
            Cedula::normalizar(&"1".repeat(21)),
            Err(CedulaInvalida::DemasiadoLarga)
        );
    }

    proptest! {
        /// Normalizar una cédula ya normalizada no la cambia: la forma
        /// única es estable, sin importar cuántas veces pase por la regla.
        #[test]
        fn normalizar_es_idempotente(texto in "[0-9A-Za-z .-]{0,24}") {
            if let Ok(cedula) = Cedula::normalizar(&texto) {
                prop_assert_eq!(Cedula::normalizar(cedula.as_str()), Ok(cedula));
            }
        }
    }
}
