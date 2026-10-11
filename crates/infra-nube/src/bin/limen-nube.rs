//! Herramienta para preparar la nube de Limen (`SurrealDB` Cloud).
//!
//! ```text
//! limen-nube probar            conecta y muestra la versión y las tablas
//! limen-nube esquema           aplica el esquema de Limen y muestra las tablas
//! limen-nube subir <carpeta>   sube la base del equipo cuya carpeta de datos
//!                              es <carpeta> (con la app cerrada)
//! ```
//!
//! La carpeta de datos de la app de escritorio en Windows es
//! `%APPDATA%\com.dqm27.limen.escritorio`: ahí están la base (`base`) y la
//! identidad del equipo (`equipo.txt`).
//!
//! Lee la conexión de las variables de entorno `LIMEN_NUBE_URL`,
//! `LIMEN_NUBE_USUARIO`, `LIMEN_NUBE_CLAVE` y, si hace falta,
//! `LIMEN_NUBE_NIVEL` (`root`, `namespace` o `base`).

use std::process::ExitCode;

use std::path::Path;

use limen_infra_nube::{ConfigNube, ErrorNube, EstadoNube, Nube};
use limen_infra_plataforma::origen_del_equipo;
use limen_infra_surreal::AlmacenSurreal;

const USO: &str = "Uso: limen-nube probar | esquema | subir <carpeta de datos>";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    limen_infra_nube::instalar_cifrado();
    let orden = std::env::args().nth(1).unwrap_or_default();
    if orden == "subir" {
        let Some(carpeta) = std::env::args().nth(2) else {
            avisar(USO);
            return ExitCode::FAILURE;
        };
        return match subir(Path::new(&carpeta)).await {
            Ok(subidos) => {
                mostrar_subida(subidos);
                ExitCode::SUCCESS
            }
            Err(error) => {
                avisar(&error);
                ExitCode::FAILURE
            }
        };
    }
    let resultado = match orden.as_str() {
        "probar" => probar().await,
        "esquema" => esquema().await,
        _ => {
            avisar(USO);
            return ExitCode::FAILURE;
        }
    };
    match resultado {
        Ok(estado) => {
            mostrar(&estado);
            ExitCode::SUCCESS
        }
        Err(error) => {
            avisar(&error.to_string());
            ExitCode::FAILURE
        }
    }
}

async fn conectar() -> Result<Nube, ErrorNube> {
    let config = ConfigNube::desde_entorno(|nombre| std::env::var(nombre).ok())?;
    Nube::conectar(&config).await
}

async fn probar() -> Result<EstadoNube, ErrorNube> {
    conectar().await?.estado().await
}

async fn esquema() -> Result<EstadoNube, ErrorNube> {
    let nube = conectar().await?;
    nube.aplicar_esquema().await?;
    nube.estado().await
}

/// Abre la base del equipo (lo que también la convierte si es de antes de
/// las unidades operativas), deja el esquema de la nube al día y sube lo que
/// falte, por el mismo camino que la app.
async fn subir(carpeta: &Path) -> Result<usize, String> {
    let ruta_equipo = carpeta.join("equipo.txt");
    if !ruta_equipo.is_file() {
        return Err(format!(
            "No está {}: ¿es la carpeta de datos de la app? Ábrela una vez antes de subir.",
            ruta_equipo.display()
        ));
    }
    let origen = origen_del_equipo(&ruta_equipo, "")?;
    let almacen = AlmacenSurreal::en_disco(&carpeta.join("base"), origen)
        .await
        .map_err(|e| format!("No se pudo abrir la base del equipo (¿está la app abierta?): {e}"))?;
    let nube = conectar().await.map_err(|e| e.to_string())?;
    nube.aplicar_esquema().await.map_err(|e| e.to_string())?;
    nube.subir_cambios(&almacen)
        .await
        .map_err(|e| e.to_string())
}

#[expect(
    clippy::print_stdout,
    reason = "es una herramienta de consola: su resultado es lo que imprime"
)]
fn mostrar_subida(subidos: usize) {
    println!("Listo: {subidos} cambios subidos a la nube.");
}

#[expect(
    clippy::print_stdout,
    reason = "es una herramienta de consola: su resultado es lo que imprime"
)]
fn mostrar(estado: &EstadoNube) {
    println!("SurrealDB {}", estado.version);
    println!(
        "{} tablas: {}",
        estado.tablas.len(),
        estado.tablas.join(", ")
    );
}

#[expect(
    clippy::print_stderr,
    reason = "es una herramienta de consola: los errores van a la salida de errores"
)]
fn avisar(mensaje: &str) {
    eprintln!("{mensaje}");
}
