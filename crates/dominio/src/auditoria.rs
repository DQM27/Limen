//! Auditoría: todo cambio queda registrado, por insignificante que parezca
//! (regla B11), también el alta.
//!
//! El dominio decide QUÉ cambió (campo, antes y después); quién lo hizo y
//! cuándo lo agrega el caso de uso, que es quien conoce la sesión y el
//! reloj. Cada entidad describe sus campos auditables una sola vez (una
//! lista de nombre y valor) y de esa misma lista salen el alta y las
//! diferencias de una edición: así un campo nuevo no puede quedar fuera de
//! una de las dos.

/// Un campo que cambió.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioCampo {
    /// Nombre estable del campo (no cambia aunque cambie la interfaz).
    pub campo: &'static str,
    /// Valor anterior; vacío en un alta.
    pub antes: String,
    pub despues: String,
}

/// Campos auditables de una entidad, en un orden fijo: nombre y valor.
pub type CamposAuditables = Vec<(&'static str, String)>;

/// En un alta, todos los campos "cambian" de vacío a su valor inicial.
pub fn cambios_de_alta(campos: CamposAuditables) -> Vec<CambioCampo> {
    campos
        .into_iter()
        .map(|(campo, despues)| CambioCampo {
            campo,
            antes: String::new(),
            despues,
        })
        .collect()
}

/// Sólo los campos cuyo valor cambió entre `antes` y `despues`, que deben
/// venir de la misma entidad (mismos campos, mismo orden).
pub fn diferencias(antes: CamposAuditables, despues: CamposAuditables) -> Vec<CambioCampo> {
    debug_assert!(
        antes
            .iter()
            .map(|(campo, _)| campo)
            .eq(despues.iter().map(|(campo, _)| campo)),
        "antes y después deben listar los mismos campos en el mismo orden"
    );
    antes
        .into_iter()
        .zip(despues)
        .filter(|((_, valor_antes), (_, valor_despues))| valor_antes != valor_despues)
        .map(|((campo, antes), (_, despues))| CambioCampo {
            campo,
            antes,
            despues,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campos(nombre: &str, cedula: &str) -> CamposAuditables {
        vec![("nombre", nombre.into()), ("cedula", cedula.into())]
    }

    #[test]
    fn el_alta_registra_todos_los_campos_desde_vacio() {
        let cambios = cambios_de_alta(campos("ANA", "111111111"));
        assert_eq!(cambios.len(), 2, "un cambio por campo");
        assert!(
            cambios.iter().all(|c| c.antes.is_empty()),
            "todo parte de vacío"
        );
        assert_eq!(cambios[0].despues, "ANA");
    }

    #[test]
    fn solo_anota_lo_que_cambio() {
        let cambios = diferencias(campos("ANA", "111111111"), campos("ANA", "222222222"));
        assert_eq!(
            cambios,
            vec![CambioCampo {
                campo: "cedula",
                antes: "111111111".into(),
                despues: "222222222".into(),
            }],
            "sólo la cédula cambió"
        );
    }

    #[test]
    fn sin_cambios_no_anota_nada() {
        assert!(
            diferencias(campos("ANA", "1"), campos("ANA", "1")).is_empty(),
            "nada que auditar"
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "mismos campos")]
    fn listas_distintas_son_un_error_de_programacion() {
        let otra = vec![("empresa", String::new()), ("cedula", String::new())];
        drop(diferencias(campos("ANA", "1"), otra));
    }
}
