//! Herramienta para preparar la nube de Limen (`SurrealDB` Cloud).
//!
//! ```text
//! limen-nube probar    conecta y muestra la versión y las tablas
//! limen-nube esquema   aplica el esquema de Limen y muestra las tablas
//! ```
//!
//! Lee la conexión de las variables de entorno `LIMEN_NUBE_URL`,
//! `LIMEN_NUBE_USUARIO`, `LIMEN_NUBE_CLAVE` y, si hace falta,
//! `LIMEN_NUBE_NIVEL` (`root`, `namespace` o `base`).

use std::process::ExitCode;

use limen_infra_nube::{ConfigNube, ErrorNube, EstadoNube, Nube};

const USO: &str = "Uso: limen-nube probar | esquema";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let orden = std::env::args().nth(1).unwrap_or_default();
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
