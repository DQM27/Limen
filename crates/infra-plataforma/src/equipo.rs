//! La identidad de este equipo: en qué unidad operativa trabaja y su ID.
//!
//! Hasta que exista el bloque M (vincular el equipo con un código de un solo
//! uso), se guarda en un archivo de texto en la carpeta de datos de la app:
//!
//! ```text
//! unidad=BRISAS
//! equipo=0199c7a2-…
//! ```
//!
//! La primera vez que arranca, el equipo crea el archivo con la unidad por
//! omisión y un ID nuevo (UUID v7); después siempre lee el mismo.

use std::path::Path;

use limen_dominio::origen::{EquipoId, Origen, UnidadOperativaId};
use uuid::Uuid;

const CLAVE_UNIDAD: &str = "unidad";
const CLAVE_EQUIPO: &str = "equipo";

/// Lee la identidad del equipo de `ruta`; si el archivo no existe, la crea
/// con `unidad_por_omision` y un ID nuevo. Un archivo que existe pero no se
/// entiende es un error: no se reemplaza, porque cambiarle el ID a un equipo
/// lo haría parecer otro.
pub fn origen_del_equipo(ruta: &Path, unidad_por_omision: &str) -> Result<Origen, String> {
    match std::fs::read_to_string(ruta) {
        Ok(texto) => leer(&texto).map_err(|motivo| format!("{}: {motivo}", ruta.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let origen = Origen {
                unidad: UnidadOperativaId::nueva(unidad_por_omision).map_err(|e| e.to_string())?,
                equipo: EquipoId::desde_uuid(Uuid::now_v7()),
            };
            let guardar = || {
                if let Some(carpeta) = ruta.parent() {
                    std::fs::create_dir_all(carpeta)?;
                }
                std::fs::write(ruta, escribir(&origen))
            };
            guardar().map_err(|e| format!("no se pudo guardar {}: {e}", ruta.display()))?;
            Ok(origen)
        }
        Err(error) => Err(format!("no se pudo leer {}: {error}", ruta.display())),
    }
}

fn escribir(origen: &Origen) -> String {
    format!(
        "{CLAVE_UNIDAD}={}\n{CLAVE_EQUIPO}={}\n",
        origen.unidad, origen.equipo
    )
}

fn leer(texto: &str) -> Result<Origen, String> {
    let valor = |clave: &str| {
        texto
            .lines()
            .filter_map(|linea| linea.split_once('='))
            .find(|(nombre, _)| nombre.trim() == clave)
            .map(|(_, valor)| valor.trim().to_owned())
            .ok_or_else(|| format!("falta `{clave}`"))
    };
    let unidad = UnidadOperativaId::nueva(&valor(CLAVE_UNIDAD)?).map_err(|e| e.to_string())?;
    let equipo = Uuid::parse_str(&valor(CLAVE_EQUIPO)?)
        .map_err(|_| format!("`{CLAVE_EQUIPO}` no es un identificador válido"))?;
    Ok(Origen {
        unidad,
        equipo: EquipoId::desde_uuid(equipo),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carpeta() -> std::path::PathBuf {
        let carpeta = std::env::temp_dir().join(format!("limen-equipo-{}", Uuid::now_v7()));
        std::fs::create_dir_all(&carpeta).unwrap();
        carpeta
    }

    #[test]
    fn la_primera_vez_crea_la_identidad_y_despues_lee_la_misma() {
        let carpeta = carpeta();
        let ruta = carpeta.join("equipo.txt");

        let primera = origen_del_equipo(&ruta, "brisas").unwrap();
        let segunda = origen_del_equipo(&ruta, "OTRA").unwrap();

        assert_eq!(primera.unidad.as_str(), "BRISAS");
        assert_eq!(
            segunda, primera,
            "lee la guardada: la unidad por omisión ya no cuenta"
        );
        drop(std::fs::remove_dir_all(carpeta));
    }

    #[test]
    fn crea_la_carpeta_si_todavia_no_existe() {
        let carpeta = std::env::temp_dir().join(format!("limen-equipo-{}", Uuid::now_v7()));
        let ruta = carpeta.join("datos").join("equipo.txt");

        let origen = origen_del_equipo(&ruta, "BRISAS").unwrap();

        assert_eq!(origen_del_equipo(&ruta, "BRISAS").unwrap(), origen);
        drop(std::fs::remove_dir_all(carpeta));
    }

    #[test]
    fn lee_un_archivo_escrito_a_mano() {
        let origen = leer("# equipo de la portería\nequipo = 00000000-0000-0000-0000-000000000007\nunidad = cartago\n")
            .unwrap();
        assert_eq!(origen.unidad.as_str(), "CARTAGO");
        assert_eq!(origen.equipo, EquipoId::desde_uuid(Uuid::from_u128(7)));
    }

    #[test]
    fn un_archivo_que_no_se_entiende_es_error_y_no_se_reemplaza() {
        let carpeta = carpeta();
        let ruta = carpeta.join("equipo.txt");
        std::fs::write(&ruta, "unidad=BRISAS\nequipo=no-es-un-id\n").unwrap();

        let error = origen_del_equipo(&ruta, "BRISAS").unwrap_err();

        assert!(error.contains("no es un identificador"), "{error}");
        assert_eq!(
            std::fs::read_to_string(&ruta).unwrap(),
            "unidad=BRISAS\nequipo=no-es-un-id\n",
            "el archivo queda como estaba"
        );
        assert!(
            leer("equipo=00000000-0000-0000-0000-000000000007").is_err(),
            "falta la unidad"
        );
        drop(std::fs::remove_dir_all(carpeta));
    }
}
