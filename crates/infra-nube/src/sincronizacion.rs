//! La sincronización en segundo plano: mientras la app está abierta, sube a
//! la nube lo que se registra en el equipo, a los pocos segundos.
//!
//! La portería nunca espera a la red (`docs/operacion-sin-conexion.md`): esto
//! corre aparte, y si no hay conexión sólo lo anota en el registro técnico y
//! vuelve a intentar. Lo que se registró mientras tanto queda en el registro
//! de cambios del equipo y sube al volver la red.

use std::time::Duration;

use limen_infra_surreal::AlmacenSurreal;

use crate::{ConfigNube, Nube};

/// Cada cuánto se sube lo nuevo mientras hay conexión.
pub const CADA: Duration = Duration::from_secs(5);
/// Cuánto se espera para reintentar si no hay conexión o algo falló.
pub const REINTENTO: Duration = Duration::from_secs(30);

/// Sube lo que cambia en el equipo mientras la app esté abierta. No termina
/// nunca: se lanza como tarea aparte al arrancar.
pub async fn sincronizar(almacen: AlmacenSurreal, config: ConfigNube) {
    loop {
        match conectar(&config).await {
            Ok(nube) => {
                log::info!("conectado a la nube: se sube lo que cambie en el equipo");
                subir_mientras_haya_conexion(&nube, &almacen).await;
            }
            Err(error) => log::warn!("sin conexión con la nube: {error}"),
        }
        tokio::time::sleep(REINTENTO).await;
    }
}

async fn conectar(config: &ConfigNube) -> Result<Nube, crate::ErrorNube> {
    let nube = Nube::conectar(config).await?;
    nube.aplicar_esquema().await?;
    Ok(nube)
}

async fn subir_mientras_haya_conexion(nube: &Nube, almacen: &AlmacenSurreal) {
    loop {
        match nube.subir_cambios(almacen).await {
            Ok(0) => {}
            Ok(cuantos) => log::info!("subidos {cuantos} cambios a la nube"),
            Err(error) => {
                log::warn!("se cortó la subida a la nube: {error}");
                return;
            }
        }
        tokio::time::sleep(CADA).await;
    }
}
