//! Quién lleva cada gafete prestado, a partir de quién está adentro.
//!
//! Reglas de negocio (bloque F de `docs/reglas.md`):
//! - F7: el gafete prestado de un tipo lo lleva quien está adentro por la vía
//!   de ese tipo (F2: [`TipoGafete::via`]) con ese número.
//! - F8: la salida por número de gafete busca a quien lo devuelve entre los
//!   que están adentro. Si nadie lo tiene, se rechaza. Cada tipo tiene su
//!   numeración, así que un mismo número puede estar en uso en más de una
//!   vía a la vez: entonces no se adivina, se rechaza y la salida se registra
//!   por la persona.
//!
//! Las funciones reciben a cada persona adentro como `(vía, gafete, dato)`:
//! el dominio no conoce los modelos de lectura, sólo decide y devuelve el
//! dato de quien corresponde.

use std::collections::HashMap;

use crate::gafete::{NumeroGafete, TipoGafete};
use crate::presencia::Via;

/// Quién lleva cada gafete prestado de este tipo (F7), por número.
pub fn quienes_llevan<T>(
    tipo: TipoGafete,
    adentro: impl IntoIterator<Item = (Via, Option<NumeroGafete>, T)>,
) -> HashMap<NumeroGafete, T> {
    adentro
        .into_iter()
        .filter(|(via, _, _)| *via == tipo.via())
        .filter_map(|(_, gafete, dato)| gafete.map(|numero| (numero, dato)))
        .collect()
}

/// No se puede saber quién devuelve el gafete (F8).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorDevolucionGafete {
    #[error("Nadie tiene adentro el gafete {0}")]
    NadieLoTiene(NumeroGafete),
    #[error(
        "El gafete {numero} está en uso en varias vías ({}): registre la salida desde la fila de la persona",
        lista_de_vias(.vias)
    )]
    EnVariasVias {
        numero: NumeroGafete,
        vias: Vec<Via>,
    },
}

impl ErrorDevolucionGafete {
    pub const fn codigo(&self) -> &'static str {
        match self {
            Self::NadieLoTiene(_) => "gafete_sin_portador",
            Self::EnVariasVias { .. } => "gafete_en_varias_vias",
        }
    }
}

fn lista_de_vias(vias: &[Via]) -> String {
    vias.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Quién devuelve el gafete con este número (F8), entre los que están
/// adentro. Para buscar en una sola vía, el caso de uso entrega sólo a los
/// de esa vía.
pub fn quien_devuelve<T>(
    numero: NumeroGafete,
    adentro: impl IntoIterator<Item = (Via, Option<NumeroGafete>, T)>,
) -> Result<(Via, T), ErrorDevolucionGafete> {
    let mut lo_tienen: Vec<(Via, T)> = adentro
        .into_iter()
        .filter(|(_, gafete, _)| *gafete == Some(numero))
        .map(|(via, _, dato)| (via, dato))
        .collect();
    if lo_tienen.len() > 1 {
        return Err(ErrorDevolucionGafete::EnVariasVias {
            numero,
            vias: lo_tienen.iter().map(|(via, _)| *via).collect(),
        });
    }
    lo_tienen
        .pop()
        .ok_or(ErrorDevolucionGafete::NadieLoTiene(numero))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numero(valor: u32) -> NumeroGafete {
        NumeroGafete::nuevo(valor).unwrap()
    }

    /// Ana entró como contratista con el 7, Beto como proveedor con el 7,
    /// Carla por correo con el 3 y Dani como contratista sin gafete.
    fn adentro() -> Vec<(Via, Option<NumeroGafete>, &'static str)> {
        vec![
            (Via::Contratista, Some(numero(7)), "ana"),
            (Via::Proveedor, Some(numero(7)), "beto"),
            (Via::Correo, Some(numero(3)), "carla"),
            (Via::Contratista, None, "dani"),
        ]
    }

    #[test]
    fn el_gafete_de_un_tipo_lo_lleva_quien_esta_adentro_por_su_via() {
        let contratista = quienes_llevan(TipoGafete::Contratista, adentro());
        assert_eq!(contratista.len(), 1, "sólo Ana: Dani no lleva gafete");
        assert_eq!(contratista.get(&numero(7)), Some(&"ana"));

        let visita = quienes_llevan(TipoGafete::Visita, adentro());
        assert_eq!(
            visita.get(&numero(3)),
            Some(&"carla"),
            "el de visita es del ingreso por correo (F2)"
        );
        assert!(
            quienes_llevan(TipoGafete::ProvisionalKof, adentro()).is_empty(),
            "nadie del KOF adentro"
        );
    }

    #[test]
    fn devuelve_el_gafete_quien_lo_tiene() {
        assert_eq!(
            quien_devuelve(numero(3), adentro()),
            Ok((Via::Correo, "carla"))
        );
    }

    #[test]
    fn un_gafete_que_nadie_tiene_se_rechaza() {
        let error = quien_devuelve(numero(9), adentro()).unwrap_err();
        assert_eq!(error, ErrorDevolucionGafete::NadieLoTiene(numero(9)));
        assert_eq!(error.codigo(), "gafete_sin_portador");
        assert_eq!(error.to_string(), "Nadie tiene adentro el gafete 9");
    }

    #[test]
    fn un_numero_en_uso_en_varias_vias_no_se_adivina() {
        let error = quien_devuelve(numero(7), adentro()).unwrap_err();
        assert_eq!(
            error,
            ErrorDevolucionGafete::EnVariasVias {
                numero: numero(7),
                vias: vec![Via::Contratista, Via::Proveedor],
            }
        );
        assert_eq!(error.codigo(), "gafete_en_varias_vias");
        assert!(
            error.to_string().contains("(contratista, proveedor)"),
            "nombra las vías: {error}"
        );
    }

    #[test]
    fn dentro_de_una_sola_via_el_numero_no_choca() {
        let solo_proveedores = adentro()
            .into_iter()
            .filter(|(via, _, _)| *via == Via::Proveedor);
        assert_eq!(
            quien_devuelve(numero(7), solo_proveedores),
            Ok((Via::Proveedor, "beto"))
        );
    }
}
