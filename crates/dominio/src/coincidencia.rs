//! Dónde aparece una palabra escrita dentro de un texto.
//!
//! Es la regla común del resaltado ([`crate::resaltado`]) y del filtro de
//! tablas ([`crate::filtro_de_tabla`]): las dos usan este mismo código, así
//! que lo que se resalta es exactamente lo que hizo que la fila coincidiera.
//! Una palabra se busca, en este orden, como palabra completa, como comienzo
//! de palabra, dentro de una palabra y, por último, perdonando errores de
//! tecleo (ver [`crate::busqueda`]).

use std::ops::Range;

use crate::busqueda::{casi_igual, plegar};

/// El texto plegado, letra por letra, y de qué letra del original viene cada
/// una. Plegar puede cambiar el largo ("ß" se vuelve "SS"), por eso hay que
/// recordar el origen para pintar sobre el texto original.
pub struct Plegado {
    pub letras: Vec<char>,
    pub origen: Vec<usize>,
}

pub fn plegar_con_origen(texto: &str) -> Plegado {
    let mut letras = Vec::new();
    let mut origen = Vec::new();
    for (posicion, letra) in texto.chars().enumerate() {
        for plegada in plegar(&letra.to_string()).chars() {
            letras.push(plegada);
            origen.push(posicion);
        }
    }
    Plegado { letras, origen }
}

/// Dónde coincide una palabra escrita (ya plegada) dentro del nombre
/// plegado, según el mejor nivel que encuentre; `None` si no coincide.
pub fn donde_coincide(escrita: &[char], nombre: &[char]) -> Option<Range<usize>> {
    let del_nombre = palabras_de(nombre);
    let texto_de = |rango: &Range<usize>| nombre.get(rango.clone()).unwrap_or_default();
    // 1. Una palabra completa.
    if let Some(rango) = del_nombre.iter().find(|r| texto_de(r) == escrita) {
        return Some(rango.clone());
    }
    // 2. El comienzo de una palabra.
    if let Some(rango) = del_nombre.iter().find(|r| texto_de(r).starts_with(escrita)) {
        return Some(rango.start..rango.start + escrita.len());
    }
    // 3. Dentro de una palabra.
    if let Some(inicio) = primera_aparicion(nombre, escrita) {
        return Some(inicio..inicio + escrita.len());
    }
    // 4. Con errores de tecleo: la palabra entera del nombre.
    let escrita: String = escrita.iter().collect();
    del_nombre
        .iter()
        .find(|r| casi_igual(&escrita, &texto_de(r).iter().collect::<String>()))
        .cloned()
}

/// Los rangos de cada palabra (separadas por espacios) en `letras`.
fn palabras_de(letras: &[char]) -> Vec<Range<usize>> {
    let mut palabras = Vec::new();
    let mut inicio = None;
    for (posicion, letra) in letras.iter().enumerate() {
        match (letra.is_whitespace(), inicio) {
            (false, None) => inicio = Some(posicion),
            (true, Some(desde)) => {
                palabras.push(desde..posicion);
                inicio = None;
            }
            _ => {}
        }
    }
    if let Some(desde) = inicio {
        palabras.push(desde..letras.len());
    }
    palabras
}

/// Dónde empieza la primera aparición de `aguja` en `pajar`.
pub fn primera_aparicion(pajar: &[char], aguja: &[char]) -> Option<usize> {
    if aguja.is_empty() {
        return None;
    }
    pajar
        .windows(aguja.len())
        .position(|ventana| ventana == aguja)
}
