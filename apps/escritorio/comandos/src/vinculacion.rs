//! Vincular el equipo con un código de un solo uso (bloque M, reglas M6 a M8).
//!
//! No necesita sesión (el equipo todavía no está vinculado ni nadie ha entrado) ni la
//! base: sólo la carpeta de datos de la app. Las reglas del código las decide el
//! dominio; aquí se traduce el resultado a lo que ve la interfaz.

use std::path::Path;

use limen_composicion::vinculacion::{
    ErrorDeVinculacion, PANEL_POR_OMISION, VAR_PANEL, esta_vinculado, vincular,
};
use serde::Serialize;

use crate::error::{ErrorJson, TipoErrorJson};

/// Si el equipo ya está vinculado y si es una compilación de desarrollo (que puede
/// seguir sin vincular, con los usuarios de la semilla).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EstadoVinculacionDto {
    pub vinculado: bool,
    pub desarrollo: bool,
}

pub fn estado_vinculacion(carpeta: &Path) -> EstadoVinculacionDto {
    EstadoVinculacionDto {
        vinculado: esta_vinculado(carpeta),
        desarrollo: cfg!(debug_assertions),
    }
}

/// Canjea el código y deja guardada la identidad del equipo. Después hay que reiniciar
/// la app: la base se abrió con la identidad anterior.
pub async fn vincular_equipo(carpeta: &Path, codigo: &str) -> Result<(), ErrorJson> {
    let panel = std::env::var(VAR_PANEL).unwrap_or_else(|_| PANEL_POR_OMISION.to_owned());
    match vincular(carpeta, &panel, codigo).await {
        Ok(_) => Ok(()),
        Err(error) => {
            log::warn!("no se pudo vincular el equipo: {error}");
            Err(traducir(&error))
        }
    }
}

fn traducir(error: &ErrorDeVinculacion) -> ErrorJson {
    let (codigo, mensaje, tecnico) = match error.codigo() {
        "codigo_invalido" => (
            "codigo_invalido",
            "El código debe tener 10 letras y números, sin 0, 1, I ni O",
            false,
        ),
        "codigo_inexistente" => ("codigo_inexistente", "Ese código no existe", false),
        "codigo_usado" => ("codigo_usado", "Ese código ya se usó", false),
        "codigo_vencido" => (
            "codigo_vencido",
            "El código venció: pida uno nuevo al administrador",
            false,
        ),
        "codigo_anulado" => ("codigo_anulado", "Ese código fue anulado", false),
        "sin_conexion" => (
            "sin_conexion",
            "No hay conexión con el panel. Revise la red e intente de nuevo",
            true,
        ),
        _ => (
            "no_se_pudo_vincular",
            "No se pudo vincular el equipo. Intente de nuevo",
            true,
        ),
    };
    ErrorJson {
        tipo: if tecnico {
            TipoErrorJson::Tecnico
        } else {
            TipoErrorJson::Negocio
        },
        codigo,
        mensaje: mensaje.to_owned(),
        campo: (!tecnico).then_some("codigo"),
    }
}

#[cfg(test)]
mod tests {
    use limen_dominio::vinculacion::ErrorVinculacion;

    use super::*;

    #[test]
    fn un_equipo_sin_archivos_no_esta_vinculado() {
        let carpeta = std::env::temp_dir().join("limen-sin-vincular-que-no-existe");
        assert!(!estado_vinculacion(&carpeta).vinculado);
    }

    #[test]
    fn las_reglas_del_codigo_llegan_con_su_codigo_y_junto_al_campo() {
        let error = traducir(&ErrorDeVinculacion::Rechazado("codigo_vencido".to_owned()));
        assert_eq!(error.codigo, "codigo_vencido");
        assert_eq!(error.campo, Some("codigo"));
        assert_eq!(error.tipo, TipoErrorJson::Negocio);
    }

    #[test]
    fn un_codigo_mal_formado_se_dice_con_la_regla_del_dominio() {
        let error = traducir(&ErrorDeVinculacion::Codigo(
            ErrorVinculacion::CodigoInvalido,
        ));
        assert_eq!(error.codigo, "codigo_invalido");
    }

    #[test]
    fn sin_red_es_un_error_tecnico_sin_campo() {
        let error = traducir(&ErrorDeVinculacion::Red("sin ruta".to_owned()));
        assert_eq!(error.codigo, "sin_conexion");
        assert_eq!(error.tipo, TipoErrorJson::Tecnico);
        assert_eq!(error.campo, None);
    }
}
