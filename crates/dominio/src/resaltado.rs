//! Qué parte de un texto se resalta cuando el operador busca (regla N1).
//!
//! Es la misma regla del buscador ([`crate::busqueda`]): se compara sin
//! importar mayúsculas ni tildes (la Ñ cuenta como N), y una palabra escrita
//! se resalta según lo mejor que encuentre en el texto, en este orden:
//!
//! 1. una palabra completa del nombre: se resalta entera;
//! 2. el comienzo de una palabra: se resalta lo escrito;
//! 3. dentro de una palabra ("ndez" en "HERNANDEZ"): se resalta lo escrito;
//! 4. con errores de tecleo ("sanchez" por "SANCHES"): se resalta la palabra
//!    entera, porque lo escrito no está literalmente en el texto.
//!
//! Cada palabra escrita se marca donde aparece en el texto; la que no aparece
//! en este texto simplemente no marca nada. Quién decide si una fila coincide
//! es el buscador o el filtro de la tabla, no el resaltado: una búsqueda
//! "carlos acme" encuentra a Carlos de la empresa Acme aunque "acme" no esté
//! en su nombre, y entonces "carlos" se resalta en la columna del nombre y
//! "acme" en la de la empresa.
//!
//! El resultado son [`Segmento`]s del texto **original** (con sus tildes y
//! mayúsculas), no posiciones: la interfaz sólo recorre los segmentos y
//! pinta los resaltados, sin contar caracteres ni repetir ninguna regla.

use std::ops::Range;

use crate::busqueda::{Criterio, MINIMO_DIGITOS_PARA_SUBCADENA};
use crate::coincidencia::{donde_coincide, plegar_con_origen, primera_aparicion};

/// Un tramo del texto, resaltado o no. Los segmentos de un texto, uno tras
/// otro, lo reconstruyen exacto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segmento {
    pub texto: String,
    pub resaltado: bool,
}

/// Qué clase de texto se resalta. Decide con qué regla del buscador se
/// compara lo escrito (N1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Campo {
    /// El nombre de una persona. Si lo escrito son sólo números, es una
    /// búsqueda por cédula y el nombre no se resalta.
    Nombre,
    /// La cédula o el código de empleado.
    Identificacion,
    /// El nombre de una empresa: se busca por nombre aunque se escriban
    /// números ("3M"), como en el buscador de empresas.
    Empresa,
}

impl Campo {
    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        match codigo {
            "NOMBRE" => Some(Self::Nombre),
            "IDENTIFICACION" => Some(Self::Identificacion),
            "EMPRESA" => Some(Self::Empresa),
            _ => None,
        }
    }
}

/// Cada uno de los `textos` partido en segmentos, con lo que coincide con
/// `buscado` resaltado. El criterio se arma una sola vez, así que sirve para
/// resaltar una columna entera.
pub fn resaltar(buscado: &str, campo: Campo, textos: &[String]) -> Vec<Vec<Segmento>> {
    let criterio = match campo {
        Campo::Nombre | Campo::Identificacion => Criterio::desde_texto(buscado),
        Campo::Empresa => Criterio::de_nombre(buscado),
    };
    textos
        .iter()
        .map(|texto| match campo {
            Campo::Identificacion => criterio.resaltar_identificacion(texto),
            Campo::Nombre | Campo::Empresa => criterio.resaltar_nombre(texto),
        })
        .collect()
}

impl Criterio {
    /// El `nombre` partido en segmentos, con cada palabra buscada resaltada
    /// donde aparece. Si no aparece ninguna (o el criterio es de cédula) no se
    /// resalta nada.
    pub fn resaltar_nombre(&self, nombre: &str) -> Vec<Segmento> {
        let Self::Nombre(palabras) = self else {
            return segmentar(nombre, &vec![false; nombre.chars().count()]);
        };
        let plegado = plegar_con_origen(nombre);
        let mut marcas = vec![false; nombre.chars().count()];
        for palabra in palabras {
            let escrita: Vec<char> = palabra.chars().collect();
            if let Some(rango) = donde_coincide(&escrita, &plegado.letras) {
                marcar(&mut marcas, &plegado.origen, rango);
            }
        }
        segmentar(nombre, &marcas)
    }

    /// Lo mismo para la cédula o el código. Se ignoran los separadores
    /// (guiones, puntos y espacios) para que también sirva sobre una cédula
    /// ya formateada ("1-1234-5678").
    pub fn resaltar_identificacion(&self, identificacion: &str) -> Vec<Segmento> {
        let sin_marcas = vec![false; identificacion.chars().count()];
        let Self::Identificacion(digitos) = self else {
            return segmentar(identificacion, &sin_marcas);
        };
        let escrita: Vec<char> = digitos.chars().collect();
        let mut letras = Vec::new();
        let mut origen = Vec::new();
        for (posicion, letra) in identificacion.chars().enumerate() {
            if !matches!(letra, '-' | '.' | ' ' | '\t') {
                letras.push(letra);
                origen.push(posicion);
            }
        }
        let rango = if letras == escrita || letras.starts_with(&escrita) {
            Some(0..escrita.len())
        } else if escrita.len() >= MINIMO_DIGITOS_PARA_SUBCADENA {
            primera_aparicion(&letras, &escrita).map(|inicio| inicio..inicio + escrita.len())
        } else {
            None
        };
        let mut marcas = sin_marcas;
        if let Some(rango) = rango {
            marcar(&mut marcas, &origen, rango);
        }
        segmentar(identificacion, &marcas)
    }
}

/// Marca, en el texto original, las letras de las que viene `rango` (que está
/// en posiciones del texto plegado).
fn marcar(marcas: &mut [bool], origen: &[usize], rango: Range<usize>) {
    let desde = origen.get(rango.start).copied();
    let hasta = rango
        .end
        .checked_sub(1)
        .and_then(|ultima| origen.get(ultima).copied());
    if let (Some(desde), Some(hasta)) = (desde, hasta) {
        for marca in marcas.iter_mut().skip(desde).take(hasta + 1 - desde) {
            *marca = true;
        }
    }
}

/// Junta las letras seguidas con la misma marca en segmentos.
fn segmentar(texto: &str, marcas: &[bool]) -> Vec<Segmento> {
    let mut segmentos: Vec<Segmento> = Vec::new();
    for (letra, &marcada) in texto.chars().zip(marcas) {
        match segmentos.last_mut() {
            Some(ultimo) if ultimo.resaltado == marcada => ultimo.texto.push(letra),
            _ => segmentos.push(Segmento {
                texto: letra.to_string(),
                resaltado: marcada,
            }),
        }
    }
    segmentos
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Los segmentos como `(texto, resaltado)`, para compararlos fácil.
    fn pares(segmentos: &[Segmento]) -> Vec<(&str, bool)> {
        segmentos
            .iter()
            .map(|s| (s.texto.as_str(), s.resaltado))
            .collect()
    }

    fn nombre(buscado: &str, nombre: &str) -> Vec<Segmento> {
        Criterio::desde_texto(buscado).resaltar_nombre(nombre)
    }

    fn reconstruido(segmentos: &[Segmento]) -> String {
        segmentos.iter().map(|s| s.texto.as_str()).collect()
    }

    #[test]
    fn el_comienzo_de_cada_palabra_se_resalta_y_conserva_las_tildes() {
        assert_eq!(
            pares(&nombre("jos pe", "José Peña")),
            [("Jos", true), ("é ", false), ("Pe", true), ("ña", false)],
            "'jos pe' marca 'Jos' y 'Pe' del original, con su tilde y su Ñ"
        );
    }

    #[test]
    fn la_ene_cuenta_como_n_y_se_resalta_la_palabra_completa() {
        assert_eq!(
            pares(&nombre("pena", "PEÑA")),
            [("PEÑA", true)],
            "'pena' encuentra a 'PEÑA'"
        );
    }

    #[test]
    fn una_palabra_completa_se_resalta_entera() {
        assert_eq!(
            pares(&nombre("carlos", "Carlos Mauricio Sanches")),
            [("Carlos", true), (" Mauricio Sanches", false)],
            "sólo la palabra escrita"
        );
    }

    #[test]
    fn las_palabras_pueden_estar_en_cualquier_orden() {
        assert_eq!(
            pares(&nombre("sanches carlos", "Carlos Mauricio Sanches")),
            [("Carlos", true), (" Mauricio ", false), ("Sanches", true)],
            "el orden en que se escribe no importa"
        );
    }

    #[test]
    fn dentro_de_una_palabra_se_resalta_solo_lo_escrito() {
        assert_eq!(
            pares(&nombre("ndez", "Hernandez")),
            [("Herna", false), ("ndez", true)],
            "una subcadena marca únicamente esa parte"
        );
    }

    #[test]
    fn con_errores_de_tecleo_se_resalta_la_palabra_entera() {
        assert_eq!(
            pares(&nombre("sanchez", "Carlos Sanches")),
            [("Carlos ", false), ("Sanches", true)],
            "'sanchez' no está escrito en el nombre: se marca la palabra parecida"
        );
    }

    #[test]
    fn solo_se_resaltan_las_palabras_que_aparecen_en_el_texto() {
        assert_eq!(
            pares(&nombre("carlos zzz", "Carlos Mauricio")),
            [("Carlos", true), (" Mauricio", false)],
            "la palabra que no está no marca nada; las demás sí"
        );
        assert_eq!(
            pares(&nombre("zzz", "Carlos Mauricio")),
            [("Carlos Mauricio", false)],
            "si ninguna aparece, no se resalta nada"
        );
    }

    #[test]
    fn una_letra_que_cambia_de_largo_al_plegarse_no_descuadra_el_resaltado() {
        // "ß" se vuelve "SS" al plegar: el origen debe seguir apuntando a la
        // misma letra del texto original.
        assert_eq!(
            pares(&nombre("strasse", "Straße")),
            [("Straße", true)],
            "toda la palabra, sin cortar la 'ß'"
        );
        assert_eq!(
            pares(&nombre("se", "Straße")),
            [("Stra", false), ("ße", true)],
            "una parte que empieza en medio de la 'SS' marca la 'ß' entera"
        );
    }

    #[test]
    fn un_criterio_vacio_o_de_cedula_no_resalta_el_nombre() {
        for buscado in ["", "   ", "1234"] {
            assert_eq!(
                pares(&nombre(buscado, "Carlos")),
                [("Carlos", false)],
                "'{buscado}' no resalta nombres"
            );
        }
        assert!(
            nombre("carlos", "").is_empty(),
            "un nombre vacío no tiene segmentos"
        );
    }

    #[test]
    fn los_segmentos_siempre_reconstruyen_el_texto_original() {
        let casos = [
            ("jos pe", "José Peña"),
            ("a", "Álvaro   Núñez"),
            ("ñ", "Muñoz"),
            ("sanchez", "Sanches"),
            ("x", "Ana"),
            ("ss", "Straße"),
        ];
        for (buscado, texto) in casos {
            assert_eq!(
                reconstruido(&nombre(buscado, texto)),
                texto,
                "'{buscado}' sobre '{texto}'"
            );
        }
    }

    #[test]
    fn si_el_buscador_dice_que_coincide_siempre_hay_algo_resaltado() {
        let nombres = [
            "José Peña",
            "Carlos Mauricio Sanches",
            "Hernandez",
            "Ana",
            "PEÑA",
        ];
        let buscados = [
            "jos",
            "pena",
            "sanchez",
            "ndez",
            "zzz",
            "carlos san",
            "an",
            "x",
        ];
        for texto in nombres {
            for buscado in buscados {
                let criterio = Criterio::desde_texto(buscado);
                if criterio.nivel("", texto).is_some() {
                    assert!(
                        criterio.resaltar_nombre(texto).iter().any(|s| s.resaltado),
                        "'{buscado}' encuentra a '{texto}': debe haber algo resaltado"
                    );
                }
            }
        }
    }

    fn cedula(buscado: &str, identificacion: &str) -> Vec<Segmento> {
        Criterio::desde_texto(buscado).resaltar_identificacion(identificacion)
    }

    #[test]
    fn la_cedula_exacta_o_por_el_comienzo_se_resalta_desde_el_inicio() {
        assert_eq!(
            pares(&cedula("112340000", "112340000")),
            [("112340000", true)],
            "exacta"
        );
        assert_eq!(
            pares(&cedula("1123", "112340000")),
            [("1123", true), ("40000", false)],
            "por el comienzo"
        );
    }

    #[test]
    fn dentro_de_la_cedula_solo_con_cuatro_digitos_o_mas() {
        assert_eq!(
            pares(&cedula("3400", "112340000")),
            [("112", false), ("3400", true), ("00", false)],
            "cuatro dígitos buscan dentro"
        );
        assert_eq!(
            pares(&cedula("340", "112340000")),
            [("112340000", false)],
            "con tres dígitos sólo se busca por el comienzo, como el buscador"
        );
    }

    #[test]
    fn la_cedula_con_formato_se_resalta_sin_contar_los_guiones() {
        assert_eq!(
            pares(&cedula("11234", "1-1234-0000")),
            [("1-1234", true), ("-0000", false)],
            "los guiones del medio quedan dentro de lo resaltado"
        );
        assert_eq!(
            reconstruido(&cedula("11234", "1-1234-0000")),
            "1-1234-0000",
            "y el texto se conserva tal cual"
        );
    }

    #[test]
    fn cada_campo_usa_la_regla_que_le_toca() {
        let textos = vec!["José Peña".to_owned(), "112340000".to_owned()];
        let marcado = |campo, buscado| -> Vec<bool> {
            resaltar(buscado, campo, &textos)
                .iter()
                .map(|segmentos| segmentos.iter().any(|s| s.resaltado))
                .collect()
        };
        assert_eq!(
            marcado(Campo::Nombre, "jos"),
            [true, false],
            "un nombre escrito resalta el nombre y no la cédula"
        );
        assert_eq!(
            marcado(Campo::Identificacion, "1123"),
            [false, true],
            "sólo números resaltan la cédula y no el nombre"
        );
        assert_eq!(
            marcado(Campo::Nombre, "1123"),
            [false, false],
            "números no resaltan un nombre de persona"
        );
        assert_eq!(
            marcado(Campo::Empresa, "112"),
            [false, true],
            "una empresa se busca por nombre aunque se escriban números"
        );
    }

    #[test]
    fn los_codigos_de_campo_son_estables() {
        assert_eq!(Campo::desde_codigo("NOMBRE"), Some(Campo::Nombre), "nombre");
        assert_eq!(
            Campo::desde_codigo("IDENTIFICACION"),
            Some(Campo::Identificacion),
            "identificación"
        );
        assert_eq!(
            Campo::desde_codigo("EMPRESA"),
            Some(Campo::Empresa),
            "empresa"
        );
        assert_eq!(
            Campo::desde_codigo("nombre"),
            None,
            "en mayúsculas, como las vías"
        );
        assert_eq!(Campo::desde_codigo(""), None, "vacío");
    }

    #[test]
    fn un_criterio_de_nombre_no_resalta_la_cedula() {
        assert_eq!(
            pares(&cedula("carlos", "112340000")),
            [("112340000", false)],
            "buscar un nombre no marca números"
        );
    }
}
