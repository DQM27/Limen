//! Nombre de una persona.
//!
//! Regla de negocio: el nombre se guarda en MAYÚSCULAS, sin espacios de
//! más y sólo con letras de la A a la Z, la Ñ y espacios. Las tildes y la
//! diéresis no se rechazan: se quitan solas ("José" → "JOSE"), así el
//! operador no pierde tiempo y el dato queda uniforme. Números y símbolos
//! (incluidos apóstrofo y guion) sí se rechazan.

use std::fmt;

/// Nombre ya validado y normalizado. Sólo se construye con
/// [`NombrePersona::nuevo`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NombrePersona(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NombreInvalido {
    #[error("el nombre está vacío")]
    Vacio,
    #[error("el nombre sólo puede tener letras y espacios")]
    CaracteresNoPermitidos,
}

impl NombrePersona {
    /// `"  josé  peña "` → `"JOSE PEÑA"`.
    pub fn nuevo(texto: &str) -> Result<Self, NombreInvalido> {
        let limpio = normalizar(texto);
        if limpio.is_empty() {
            return Err(NombreInvalido::Vacio);
        }
        if !limpio.chars().all(es_caracter_permitido) {
            return Err(NombreInvalido::CaracteresNoPermitidos);
        }
        Ok(Self(limpio))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NombrePersona {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lo que la interfaz muestra mientras se escribe: la misma normalización
/// que al guardar, pero sin tocar los espacios (si no, no se podría
/// escribir el espacio entre nombre y apellido).
pub fn nombre_mientras_se_escribe(texto: &str) -> String {
    texto.to_uppercase().chars().map(sin_tilde).collect()
}

fn normalizar(texto: &str) -> String {
    texto
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
        .chars()
        .map(sin_tilde)
        .collect()
}

/// Quita tildes y diéresis de una letra ya en mayúscula. La Ñ no es una N
/// con tilde: es otra letra y se conserva.
const fn sin_tilde(letra: char) -> char {
    match letra {
        'Á' | 'À' | 'Ä' | 'Â' => 'A',
        'É' | 'È' | 'Ë' | 'Ê' => 'E',
        'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
        'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
        'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
        otra => otra,
    }
}

const fn es_caracter_permitido(c: char) -> bool {
    c.is_ascii_uppercase() || c == 'Ñ' || c == ' '
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn nombre(texto: &str) -> String {
        NombrePersona::nuevo(texto).unwrap().as_str().to_string()
    }

    #[test]
    fn queda_en_mayusculas_y_sin_espacios_de_mas() {
        assert_eq!(nombre("  ana  solano "), "ANA SOLANO");
    }

    #[test]
    fn quita_las_tildes_y_conserva_la_ene() {
        assert_eq!(nombre("josé peña"), "JOSE PEÑA");
        assert_eq!(nombre("MARÍA AGÜERO"), "MARIA AGUERO");
        assert_eq!(nombre("Ñandú Íñiguez"), "ÑANDU IÑIGUEZ");
    }

    #[test]
    fn rechaza_vacio_numeros_y_simbolos() {
        assert_eq!(NombrePersona::nuevo("   "), Err(NombreInvalido::Vacio));
        for invalido in ["Ana 2", "Ana@", "O'Neil", "Solano-Rojas", "Ana.", "Anaç"] {
            assert_eq!(
                NombrePersona::nuevo(invalido),
                Err(NombreInvalido::CaracteresNoPermitidos),
                "{invalido}"
            );
        }
    }

    #[test]
    fn mientras_se_escribe_conserva_el_espacio_del_final() {
        assert_eq!(nombre_mientras_se_escribe("josé "), "JOSE ");
        assert_eq!(nombre_mientras_se_escribe("peña"), "PEÑA");
    }

    proptest! {
        #[test]
        fn normalizar_es_idempotente(texto in "[a-zA-ZáéíóúüñÑ ]{0,30}") {
            if let Ok(nombre) = NombrePersona::nuevo(&texto) {
                prop_assert_eq!(NombrePersona::nuevo(nombre.as_str()), Ok(nombre));
            }
        }
    }
}
