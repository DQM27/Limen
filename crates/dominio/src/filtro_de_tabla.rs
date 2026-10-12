//! Qué filas pasan el buscador de una tabla y qué celdas cumplen un filtro de
//! columna (regla N2).
//!
//! Antes esto vivía duplicado en la pantalla. Ahora la pantalla sólo pregunta
//! y aplica la respuesta: no pliega, no compara ni parte por palabras.
//!
//! # El buscador de la tabla
//!
//! Es el mismo del núcleo ([`crate::busqueda`]), repartido entre las columnas
//! de la fila: cada palabra escrita debe aparecer en **alguna** columna, en
//! cualquier orden. "carlos acme" encuentra a Carlos de la empresa Acme aunque
//! "acme" no esté en su nombre. Cada palabra se busca con las reglas de
//! siempre (palabra completa, comienzo, dentro de una palabra, o perdonando
//! errores de tecleo). Si lo escrito son sólo números, se busca por cédula o
//! código, y también dentro de la empresa. Un buscador vacío no excluye a nadie.
//!
//! Se usa la misma coincidencia que el resaltado ([`crate::coincidencia`]), así
//! que lo resaltado es exactamente lo que hizo que la fila coincidiera.
//!
//! # Los filtros de columna
//!
//! Se compara sin importar mayúsculas ni tildes (la Ñ cuenta como N). "Contiene"
//! parte por palabras y acepta cualquier orden ("carlos sa" encuentra a "Carlos
//! Mauricio Sánchez"); "no contiene" es su contrario. Las demás operaciones
//! comparan una sola frase. Sin texto de filtro, todo coincide.

use crate::busqueda::{Criterio, plegar};
use crate::coincidencia::{Plegado, donde_coincide, plegar_con_origen};
use crate::resaltado::Campo;

/// Un texto de la fila y de qué clase es (decide con qué regla se compara).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampoDeFila<'a> {
    pub campo: Campo,
    pub texto: &'a str,
}

/// Si una fila coincide con lo escrito en el buscador de la tabla.
pub fn fila_coincide(buscado: &str, campos: &[CampoDeFila<'_>]) -> bool {
    let criterio = Criterio::desde_texto(buscado);
    match &criterio {
        Criterio::Vacio => true,
        Criterio::Identificacion(_) => {
            let por_cedula = campos
                .iter()
                .any(|c| c.campo == Campo::Identificacion && criterio.nivel(c.texto, "").is_some());
            por_cedula || palabras_en(&Criterio::de_nombre(buscado), campos, &[Campo::Empresa])
        }
        Criterio::Nombre(_) => palabras_en(&criterio, campos, &[Campo::Nombre, Campo::Empresa]),
    }
}

/// Si cada palabra del criterio aparece en alguno de los campos de las clases
/// indicadas.
fn palabras_en(criterio: &Criterio, campos: &[CampoDeFila<'_>], clases: &[Campo]) -> bool {
    let Criterio::Nombre(palabras) = criterio else {
        return false;
    };
    let textos: Vec<Plegado> = campos
        .iter()
        .filter(|c| clases.contains(&c.campo))
        .map(|c| plegar_con_origen(c.texto))
        .collect();
    palabras.iter().all(|palabra| {
        let escrita: Vec<char> = palabra.chars().collect();
        textos
            .iter()
            .any(|texto| donde_coincide(&escrita, &texto.letras).is_some())
    })
}

/// Lo que se le puede pedir a un filtro de texto de columna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operacion {
    Contiene,
    NoContiene,
    Igual,
    Distinto,
    EmpiezaCon,
    TerminaCon,
}

impl Operacion {
    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        match codigo {
            "CONTIENE" => Some(Self::Contiene),
            "NO_CONTIENE" => Some(Self::NoContiene),
            "IGUAL" => Some(Self::Igual),
            "DISTINTO" => Some(Self::Distinto),
            "EMPIEZA_CON" => Some(Self::EmpiezaCon),
            "TERMINA_CON" => Some(Self::TerminaCon),
            _ => None,
        }
    }
}

/// Si la `celda` cumple el filtro de columna: `operacion` con lo escrito en
/// `filtro`.
pub fn cumple_filtro_de_columna(operacion: Operacion, celda: &str, filtro: &str) -> bool {
    let filtro = plegar(filtro);
    if filtro.trim().is_empty() {
        return true;
    }
    let celda = plegar(celda);
    match operacion {
        Operacion::Contiene => contiene_palabras(&celda, &filtro),
        Operacion::NoContiene => !contiene_palabras(&celda, &filtro),
        Operacion::Igual => celda == filtro,
        Operacion::Distinto => celda != filtro,
        Operacion::EmpiezaCon => celda.starts_with(&filtro),
        Operacion::TerminaCon => celda.ends_with(&filtro),
    }
}

fn contiene_palabras(celda: &str, filtro: &str) -> bool {
    filtro
        .split_whitespace()
        .all(|palabra| celda.contains(palabra))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resaltado::resaltar;

    use Campo::{Empresa, Identificacion, Nombre};

    fn fila<'a>(cedula: &'a str, nombre: &'a str, empresa: &'a str) -> Vec<CampoDeFila<'a>> {
        vec![
            CampoDeFila {
                campo: Identificacion,
                texto: cedula,
            },
            CampoDeFila {
                campo: Nombre,
                texto: nombre,
            },
            CampoDeFila {
                campo: Empresa,
                texto: empresa,
            },
        ]
    }

    fn carlos() -> Vec<CampoDeFila<'static>> {
        fila("112340000", "Carlos Mauricio Sánchez Aldama", "Acme S.A.")
    }

    #[test]
    fn el_buscador_encuentra_por_palabras_en_cualquier_orden() {
        let fila = carlos();
        assert!(
            fila_coincide("sanchez carlos", &fila),
            "orden libre y sin tildes"
        );
        assert!(
            fila_coincide("carlos sa", &fila),
            "el comienzo de una palabra"
        );
        assert!(
            !fila_coincide("carlos rojas", &fila),
            "una palabra que no está"
        );
    }

    #[test]
    fn las_palabras_pueden_repartirse_entre_columnas() {
        let fila = carlos();
        assert!(fila_coincide("carlos acme", &fila), "nombre y empresa");
        assert!(
            !fila_coincide("carlos zeta", &fila),
            "ninguna columna tiene 'zeta'"
        );
    }

    #[test]
    fn un_buscador_vacio_no_excluye_a_nadie() {
        assert!(fila_coincide("", &carlos()), "vacío");
        assert!(fila_coincide("   ", &carlos()), "sólo espacios");
        assert!(fila_coincide("", &[]), "ni siquiera una fila sin columnas");
    }

    #[test]
    fn perdona_errores_de_tecleo_como_el_buscador_del_nucleo() {
        let fila = fila("1", "Carlos Sanches", "Acme");
        assert!(fila_coincide("sanchez", &fila), "'sanchez' por 'SANCHES'");
        assert!(!fila_coincide("sanxxxx", &fila), "demasiado distinto");
    }

    #[test]
    fn solo_numeros_busca_por_cedula_y_tambien_en_la_empresa() {
        let la_fila = carlos();
        assert!(fila_coincide("1123", &la_fila), "el comienzo de la cédula");
        assert!(fila_coincide("1-1234-0000", &la_fila), "con separadores");
        assert!(!fila_coincide("999", &la_fila), "una cédula que no es");
        assert!(
            !fila_coincide("1123", &fila("999", "Ana 1123", "Zeta")),
            "los números no buscan en el nombre de la persona"
        );
        assert!(
            fila_coincide("3", &fila("999", "Ana", "3M")),
            "pero sí en la empresa ('3M')"
        );
    }

    #[test]
    fn una_columna_sin_clase_de_busqueda_no_cuenta_y_sin_columnas_no_hay_coincidencia() {
        assert!(!fila_coincide("carlos", &[]), "sin columnas que mirar");
    }

    #[test]
    fn si_la_fila_coincide_cada_palabra_queda_resaltada_en_alguna_columna() {
        let fila = carlos();
        for buscado in ["sanchez carlos", "carlos acme", "carlos sa", "mauricio"] {
            assert!(fila_coincide(buscado, &fila), "'{buscado}' debe coincidir");
            for palabra in buscado.split_whitespace() {
                let resaltada = fila.iter().any(|c| {
                    resaltar(palabra, c.campo, &[c.texto.to_owned()])
                        .iter()
                        .flatten()
                        .any(|s| s.resaltado)
                });
                assert!(
                    resaltada,
                    "'{palabra}' de '{buscado}' coincide: debe haber algo resaltado"
                );
            }
        }
    }

    fn filtro(operacion: &str, celda: &str, texto: &str) -> bool {
        cumple_filtro_de_columna(
            Operacion::desde_codigo(operacion).expect("código de operación válido"),
            celda,
            texto,
        )
    }

    #[test]
    fn contiene_parte_por_palabras_y_no_distingue_tildes() {
        assert!(
            filtro("CONTIENE", "Carlos Mauricio Sánchez", "carlos sa"),
            "palabras"
        );
        assert!(
            filtro("CONTIENE", "Carlos Mauricio Sánchez", "sanchez"),
            "sin tildes"
        );
        assert!(
            !filtro("CONTIENE", "Carlos Mauricio Sánchez", "ana"),
            "no está"
        );
        assert!(filtro("NO_CONTIENE", "Carlos", "ana"), "no contiene");
        assert!(
            !filtro("NO_CONTIENE", "Carlos Mauricio Sánchez", "carlos sa"),
            "no contiene es lo contrario de contiene"
        );
    }

    #[test]
    fn las_demas_operaciones_comparan_una_sola_frase() {
        assert!(filtro("IGUAL", "Ana María", "ANA MARIA"), "igual");
        assert!(filtro("DISTINTO", "Ana", "beto"), "distinto");
        assert!(!filtro("DISTINTO", "Ana", "ANA"), "no es distinto");
        assert!(
            filtro("EMPIEZA_CON", "Ñandú", "nan"),
            "empieza con, la Ñ cuenta como N"
        );
        assert!(filtro("TERMINA_CON", "Ñandú", "ndu"), "termina con");
        assert!(!filtro("EMPIEZA_CON", "Ñandú", "andu"), "no empieza con");
    }

    #[test]
    fn sin_texto_de_filtro_todo_coincide() {
        for operacion in [
            "CONTIENE",
            "NO_CONTIENE",
            "IGUAL",
            "DISTINTO",
            "EMPIEZA_CON",
        ] {
            assert!(filtro(operacion, "x", ""), "{operacion} con filtro vacío");
            assert!(filtro(operacion, "x", "  "), "{operacion} con espacios");
        }
    }

    #[test]
    fn las_operaciones_tienen_codigo_estable() {
        assert_eq!(
            Operacion::desde_codigo("CONTIENE"),
            Some(Operacion::Contiene),
            "contiene"
        );
        assert_eq!(
            Operacion::desde_codigo("contains"),
            None,
            "no es un código nuestro"
        );
        assert_eq!(Operacion::desde_codigo(""), None, "vacío");
    }
}
