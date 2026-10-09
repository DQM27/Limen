//! `importar-lattis --datos <carpeta del volcado> --base <carpeta de la base>`
//!
//! Carga el volcado de Lattis en una base de Limen. Cierre la app de
//! escritorio antes: la base queda tomada mientras la app esté abierta. Por
//! omisión carga en la base de la app de escritorio de este equipo.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use limen_aplicacion::puertos::Reloj;
use limen_importador_lattis::{Datos, importar};
use limen_infra_plataforma::{IdsV7, RelojCostaRica};
use limen_infra_surreal::AlmacenSurreal;

const CARPETA_DATOS: &str = "datos-privados/lattis-produccion";
const CARPETA_APP: &str = "com.dqm27.limen.escritorio";

const AYUDA: &str = "\
Uso: importar-lattis [--datos <carpeta>] [--base <carpeta>]

  --datos  carpeta con los 01_*.sql … 09_*.sql del volcado
           (por omisión: datos-privados/lattis-produccion)
  --base   carpeta de la base de Limen a llenar
           (por omisión: la de la app de escritorio de este equipo)

Cierre la app de escritorio antes de importar. Si la base ya tenía datos,
bórrela primero: el importador no mezcla con lo que ya hay.";

struct Opciones {
    datos: PathBuf,
    base: PathBuf,
}

fn leer_opciones() -> Result<Option<Opciones>, String> {
    let mut datos = PathBuf::from(CARPETA_DATOS);
    let mut base = std::env::var_os("APPDATA")
        .map(|appdata| PathBuf::from(appdata).join(CARPETA_APP).join("base"));
    let mut argumentos = std::env::args().skip(1);
    while let Some(argumento) = argumentos.next() {
        match argumento.as_str() {
            "--ayuda" | "-h" => return Ok(None),
            "--datos" => datos = argumentos.next().ok_or("falta la ruta de --datos")?.into(),
            "--base" => base = Some(argumentos.next().ok_or("falta la ruta de --base")?.into()),
            otro => return Err(format!("no entiendo `{otro}`")),
        }
    }
    let base = base.ok_or("no sé dónde está la base: use --base <carpeta>")?;
    Ok(Some(Opciones { datos, base }))
}

async fn ejecutar() -> Result<(), String> {
    let Some(opciones) = leer_opciones()? else {
        drop(writeln!(std::io::stdout(), "{AYUDA}"));
        return Ok(());
    };
    let datos = Datos::desde_carpeta(&opciones.datos).map_err(|error| error.to_string())?;
    drop(writeln!(
        std::io::stdout(),
        "Importando de {} a {}",
        opciones.datos.display(),
        opciones.base.display()
    ));
    let almacen = AlmacenSurreal::en_disco(&opciones.base)
        .await
        .map_err(|error| format!("no se pudo abrir la base (¿está abierta la app?): {error}"))?;
    let resumen = importar(&almacen, &IdsV7, RelojCostaRica.ahora(), &datos)
        .await
        .map_err(|error| error.to_string())?;
    drop(writeln!(std::io::stdout(), "\n{resumen}"));
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match ejecutar().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(mensaje) => {
            drop(writeln!(std::io::stderr(), "error: {mensaje}"));
            ExitCode::FAILURE
        }
    }
}
