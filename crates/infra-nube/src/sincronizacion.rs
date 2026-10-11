//! La sincronización en segundo plano: mientras la app está abierta, sube a
//! la nube lo que se registra en el equipo y baja lo que registran los demás
//! equipos, cada pocos segundos.
//!
//! En cada vuelta primero sube y después baja: así, si el equipo cambió algo
//! sin conexión, la nube recibe su versión antes de que el equipo repase lo
//! que cambió allá, y el repaso termina en lo más reciente.
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

/// Sube y baja lo que cambia mientras la app esté abierta. No termina
/// nunca: se lanza como tarea aparte al arrancar. `al_llegar` se llama cada
/// vez que bajó algo que cambió el equipo (para que las pantallas se
/// actualicen).
pub async fn sincronizar<F>(almacen: AlmacenSurreal, config: ConfigNube, al_llegar: F)
where
    F: Fn() + Send + Sync + 'static,
{
    loop {
        match conectar(&config).await {
            Ok(nube) => {
                log::info!("conectado a la nube: se sincroniza lo que cambie");
                sincronizar_mientras_haya_conexion(&nube, &almacen, &al_llegar).await;
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

async fn sincronizar_mientras_haya_conexion<F: Fn() + Sync>(
    nube: &Nube,
    almacen: &AlmacenSurreal,
    al_llegar: &F,
) {
    loop {
        match nube.subir_cambios(almacen).await {
            Ok(0) => {}
            Ok(cuantos) => log::info!("subidos {cuantos} cambios a la nube"),
            Err(error) => {
                log::warn!("se cortó la subida a la nube: {error}");
                return;
            }
        }
        match nube.bajar_cambios(almacen).await {
            Ok(0) => {}
            Ok(cuantas) => {
                log::info!("bajadas {cuantas} filas de la nube");
                al_llegar();
            }
            Err(error) => {
                log::warn!("se cortó la bajada de la nube: {error}");
                return;
            }
        }
        tokio::time::sleep(CADA).await;
    }
}
