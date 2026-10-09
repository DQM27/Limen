//! Auditoría: todo cambio queda registrado, por insignificante que parezca.
//!
//! El dominio decide QUÉ cambió (campo, antes y después); quién lo hizo y
//! cuándo lo agrega el caso de uso, que es quien conoce la sesión y el
//! reloj.

/// Un campo que cambió en una edición.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioCampo {
    /// Nombre estable del campo (no cambia aunque cambie la interfaz).
    pub campo: &'static str,
    pub antes: String,
    pub despues: String,
}

/// Agrega el cambio a `cambios` sólo si el valor de verdad cambió.
pub(crate) fn anotar_si_cambia<T: PartialEq + ToString>(
    cambios: &mut Vec<CambioCampo>,
    campo: &'static str,
    antes: &T,
    despues: &T,
) {
    if antes != despues {
        cambios.push(CambioCampo {
            campo,
            antes: antes.to_string(),
            despues: despues.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_anota_lo_que_cambio() {
        let mut cambios = Vec::new();
        anotar_si_cambia(&mut cambios, "nombre", &"ANA", &"ANA");
        anotar_si_cambia(&mut cambios, "cedula", &"111111111", &"222222222");
        assert_eq!(
            cambios,
            vec![CambioCampo {
                campo: "cedula",
                antes: "111111111".into(),
                despues: "222222222".into(),
            }]
        );
    }
}
