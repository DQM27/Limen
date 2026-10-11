//! La sincronización en segundo plano: mientras la app está abierta, sube a
//! la nube lo que se registra en el equipo y baja lo que registran los demás
//! equipos, **al instante**.
//!
//! Despierta por tres cosas, lo que pase primero:
//! - la app confirmó algo en la base del equipo: lo sube en el momento;
//! - la nube avisó en vivo (`LIVE SELECT`) que otro equipo cambió algo;
//! - pasaron unos [`RED_DE_SEGURIDAD`] segundos sin avisos (con una pequeña
//!   variación al azar): por si se perdió uno.
//!
//! En cada vuelta primero sube y después baja: así, si el equipo cambió algo
//! sin conexión, la nube recibe su versión antes de que el equipo repase lo
//! que cambió allá, y el repaso termina en lo más reciente. Lo que se baja
//! sale siempre del registro de cambios, no de los avisos.
//!
//! La portería nunca espera a la red (`docs/operacion-sin-conexion.md`): esto
//! corre aparte, y si no hay conexión sólo lo anota en el registro técnico y
//! vuelve a intentar. Lo que se registró mientras tanto queda en el registro
//! de cambios del equipo y sube al volver la red.
//!
//! Para no tumbar a la nube cuando vuelve tras una caída, cada reintento de
//! conexión espera un tiempo al azar que crece con los fallos seguidos
//! (`crate::espera`, `docs/nube.md` sección 11).

use std::sync::Arc;
use std::time::Duration;

use limen_infra_surreal::AlmacenSurreal;
use tokio::sync::Notify;

use crate::espera::{self, AzarDelSistema};
use crate::{ConfigNube, Escucha, Nube};

/// Cada cuánto se sincroniza, más o menos, aunque no llegue ningún aviso.
pub const RED_DE_SEGURIDAD: Duration = Duration::from_secs(10);
/// Cuántas veces se reintenta enseguida una vuelta que falló.
const REINTENTOS_DE_VUELTA: u32 = 3;
/// Techo de la espera antes del primer reintento de conexión; cada fallo
/// seguido lo duplica hasta `espera::TOPE` y la espera real es al azar por
/// debajo de ese techo.
pub const REINTENTO: Duration = Duration::from_secs(30);

/// Sube y baja lo que cambia mientras la app esté abierta. No termina
/// nunca: se lanza como tarea aparte al arrancar. `al_llegar` se llama cada
/// vez que bajó algo que cambió el equipo (para que las pantallas se
/// actualicen).
pub async fn sincronizar<F>(almacen: AlmacenSurreal, config: ConfigNube, al_llegar: F)
where
    F: Fn() + Send + Sync + 'static,
{
    let mut azar = AzarDelSistema;
    // Fallos de conexión seguidos: hacen crecer la espera. Se reinicia al
    // conectar.
    let mut fallos: u32 = 0;
    // El esquema se aplica sólo en la primera conexión buena: reaplicarlo en
    // cada reconexión sería carga inútil para la nube.
    let mut esquema_listo = false;
    loop {
        match conectar(&config, !esquema_listo).await {
            Ok(nube) => {
                esquema_listo = true;
                fallos = 0;
                log::info!("conectado a la nube: se sincroniza al instante");
                sincronizar_mientras_haya_conexion(&nube, &almacen, &al_llegar).await;
            }
            Err(error) => log::warn!("sin conexión con la nube: {error}"),
        }
        fallos = fallos.saturating_add(1);
        let pausa = espera::con_jitter(REINTENTO, fallos, &mut azar);
        log::debug!("se reintenta la conexión con la nube en {pausa:?} (fallo {fallos})");
        tokio::time::sleep(pausa).await;
    }
}

async fn conectar(config: &ConfigNube, aplicar_esquema: bool) -> Result<Nube, crate::ErrorNube> {
    let nube = Nube::conectar(config).await?;
    if aplicar_esquema {
        nube.aplicar_esquema().await?;
    }
    Ok(nube)
}

async fn sincronizar_mientras_haya_conexion<F: Fn() + Sync>(
    nube: &Nube,
    almacen: &AlmacenSurreal,
    al_llegar: &F,
) {
    let avisos = Arc::new(Notify::new());
    let mut azar = AzarDelSistema;
    let mut escucha: Option<Escucha> = None;
    loop {
        // Sin avisos en vivo se sigue igual, con la vuelta periódica, y se
        // vuelve a intentar suscribirse en cada vuelta.
        if !escucha.as_ref().is_some_and(Escucha::sigue_activa) {
            escucha = match nube
                .escuchar(almacen.unidad().as_str(), Arc::clone(&avisos))
                .await
            {
                Ok(nueva) => Some(nueva),
                Err(error) => {
                    log::warn!("sin avisos en vivo de la nube por ahora: {error}");
                    None
                }
            };
        }
        if !una_vuelta_con_reintentos(nube, almacen, al_llegar).await {
            return;
        }
        let red_de_seguridad = espera::variar(RED_DE_SEGURIDAD, &mut azar);
        tokio::select! {
            () = almacen.esperar_confirmacion() => {}
            () = avisos.notified() => {}
            () = tokio::time::sleep(red_de_seguridad) => {}
        }
    }
}

/// Una vuelta, reintentada enseguida unas veces: si otro equipo escribía lo
/// mismo en ese momento, la nube pide reintentar. `false` si sigue fallando
/// (se cortó la conexión).
async fn una_vuelta_con_reintentos<F: Fn() + Sync>(
    nube: &Nube,
    almacen: &AlmacenSurreal,
    al_llegar: &F,
) -> bool {
    for intento in 1..=REINTENTOS_DE_VUELTA {
        if una_vuelta(nube, almacen, al_llegar).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200) * intento).await;
    }
    false
}

/// Sube y después baja. `false` si se cortó la conexión.
async fn una_vuelta<F: Fn() + Sync>(nube: &Nube, almacen: &AlmacenSurreal, al_llegar: &F) -> bool {
    match nube.subir_cambios(almacen).await {
        Ok(0) => {}
        Ok(cuantos) => log::info!("subidos {cuantos} cambios a la nube"),
        Err(error) => {
            log::warn!("se cortó la subida a la nube: {error}");
            return false;
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
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    use limen_aplicacion::puertos::{
        FabricaUnidadDeTrabajo, RepositorioPresencias, UnidadDeTrabajo,
    };
    use limen_dominio::cedula::Cedula;
    use limen_dominio::presencia::{Identidad, Via};

    use super::*;

    fn persona() -> Identidad {
        Identidad::from(&Cedula::normalizar("111111111").unwrap())
    }

    async fn adentro(equipo: &AlmacenSurreal) -> bool {
        equipo
            .nueva()
            .presencias()
            .via_adentro(&persona())
            .await
            .unwrap()
            .is_some()
    }

    /// Espera a que se cumpla `condicion`, a lo sumo `limite`; devuelve
    /// cuánto tardó.
    async fn cuanto_tarda<C, Fut>(limite: Duration, condicion: C) -> Option<Duration>
    where
        C: Fn() -> Fut,
        Fut: Future<Output = bool>,
    {
        let inicio = Instant::now();
        while inicio.elapsed() < limite {
            if condicion().await {
                return Some(inicio.elapsed());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        None
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lo_que_registra_un_equipo_aparece_en_el_otro_al_instante() {
        let nube = Nube::conectar(&ConfigNube {
            url: "mem://".to_owned(),
            credenciales: None,
        })
        .await
        .unwrap();
        nube.aplicar_esquema().await.unwrap();
        let a = AlmacenSurreal::en_memoria().await.unwrap();
        let b = AlmacenSurreal::en_memoria().await.unwrap();
        let avisos_en_b = Arc::new(AtomicUsize::new(0));
        for (equipo, avisos) in [
            (a.clone(), None),
            (b.clone(), Some(Arc::clone(&avisos_en_b))),
        ] {
            let nube = nube.clone();
            tokio::spawn(async move {
                let al_llegar = move || {
                    if let Some(avisos) = &avisos {
                        avisos.fetch_add(1, Ordering::SeqCst);
                    }
                };
                sincronizar_mientras_haya_conexion(&nube, &equipo, &al_llegar).await;
            });
        }
        // Que los dos terminen su primera vuelta.
        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut uow = a.nueva();
        uow.presencias().anotar_entrada(
            &persona(),
            Via::Contratista,
            "2026-10-09T08:00:00Z".parse().unwrap(),
        );
        uow.confirmar().await.unwrap();

        let tardo = cuanto_tarda(Duration::from_secs(5), || adentro(&b)).await;
        assert!(
            tardo.is_some_and(|tardo| tardo < Duration::from_secs(2)),
            "B lo ve sin esperar la red de seguridad de {RED_DE_SEGURIDAD:?}: tardó {tardo:?}"
        );
        let avisos_en_b = &avisos_en_b;
        assert!(
            cuanto_tarda(Duration::from_secs(1), || async move {
                avisos_en_b.load(Ordering::SeqCst) > 0
            })
            .await
            .is_some(),
            "y B avisa a sus pantallas"
        );
    }
}
