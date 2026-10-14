//! Avisos en vivo de la nube (`LIVE SELECT`): cuando otro equipo cambia algo,
//! la nube avisa al instante y la sincronización despierta, sin esperar la
//! vuelta periódica.
//!
//! Los avisos sólo despiertan: lo que se baja sale siempre del registro de
//! cambios (`bajada`). Así, si se pierde un aviso (un corte de un segundo),
//! la vuelta periódica recupera exactamente lo que faltó.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use surrealdb::types::Value;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::{ErrorNube, Nube};

/// Cuántas veces se intenta suscribirse antes de rendirse.
const INTENTOS: u32 = 5;
/// La pausa entre intentos crece: 100 ms, 200 ms…
const PAUSA_ENTRE_INTENTOS: Duration = Duration::from_millis(100);

/// La escucha en vivo de una conexión. Al soltarla se detiene.
#[derive(Debug)]
pub struct Escucha(JoinHandle<()>);

impl Escucha {
    /// Si sigue recibiendo avisos (no se cortó).
    pub fn sigue_activa(&self) -> bool {
        !self.0.is_finished()
    }
}

impl Drop for Escucha {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Nube {
    /// Se suscribe al diario de cambios: lo de `unidad` y lo global. Cada
    /// aviso de la nube despierta a `avisos`. Si la suscripción se corta,
    /// también lo despierta, para que la sincronización note el corte.
    ///
    /// Suscribirse mientras otros escriben puede chocar ("conflicto de
    /// transacción, reintente"): se reintenta unas veces antes de rendirse.
    pub async fn escuchar(&self, unidad: &str, avisos: Arc<Notify>) -> Result<Escucha, ErrorNube> {
        let mut intento = 1;
        loop {
            match self.escuchar_una_vez(unidad, Arc::clone(&avisos)).await {
                Err(error) if intento < INTENTOS => {
                    log::debug!("reintentando la suscripción en vivo: {error}");
                    tokio::time::sleep(PAUSA_ENTRE_INTENTOS * intento).await;
                    intento += 1;
                }
                resultado => return resultado,
            }
        }
    }

    async fn escuchar_una_vez(
        &self,
        unidad: &str,
        avisos: Arc<Notify>,
    ) -> Result<Escucha, ErrorNube> {
        // Un solo aviso para todo: el diario de cambios (`cambio`). Cada alta, cambio o borrado de
        // una tabla compartida deja una anotación, y cada equipo sólo recibe las de su unidad (los
        // permisos de la tabla) y las globales.
        let mut respuesta = self
            .db
            .query("LIVE SELECT * FROM cambio WHERE unidad = $unidad OR unidad = '';")
            .bind(("unidad", unidad.to_owned()))
            .await
            .map_err(|e| ErrorNube::Conexion(e.to_string()))?;
        let mut flujo = respuesta
            .stream::<Value>(())
            .map_err(|e| ErrorNube::Conexion(e.to_string()))?;
        Ok(Escucha(tokio::spawn(async move {
            while flujo.next().await.is_some() {
                avisos.notify_one();
            }
            log::warn!("se cortaron los avisos en vivo de la nube");
            avisos.notify_one();
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigNube;

    async fn nube() -> Nube {
        let nube = Nube::conectar(&ConfigNube {
            url: "mem://".to_owned(),
            credenciales: None,
        })
        .await
        .unwrap();
        nube.aplicar_esquema().await.unwrap();
        nube
    }

    async fn crear(nube: &Nube, sentencia: &str) {
        nube.db.query(sentencia).await.unwrap().check().unwrap();
    }

    async fn avisa(avisos: &Notify) -> bool {
        tokio::time::timeout(Duration::from_secs(2), avisos.notified())
            .await
            .is_ok()
    }

    #[tokio::test]
    async fn la_nube_avisa_al_instante_lo_de_la_unidad_y_los_catalogos() {
        let nube = nube().await;
        let avisos = Arc::new(Notify::new());
        let _escucha = nube.escuchar("PRUEBAS", Arc::clone(&avisos)).await.unwrap();

        crear(
            &nube,
            "CREATE presencia:`111111111` CONTENT { unidad: 'PRUEBAS', via: 'CONTRATISTA', \
             desde: d'2026-10-09T08:00:00Z' }",
        )
        .await;
        assert!(avisa(&avisos).await, "avisa lo de su unidad");

        crear(
            &nube,
            "CREATE empresa:uno CONTENT { nombre: 'ACME', nombre_busqueda: 'ACME' }",
        )
        .await;
        assert!(avisa(&avisos).await, "y los catálogos, que son de todos");
    }

    #[tokio::test]
    async fn lo_de_otra_unidad_no_despierta() {
        let nube = nube().await;
        let avisos = Arc::new(Notify::new());
        let _escucha = nube.escuchar("PRUEBAS", Arc::clone(&avisos)).await.unwrap();

        crear(
            &nube,
            "CREATE presencia:`222222222` CONTENT { unidad: 'OTRA', via: 'PROVEEDOR', \
             desde: d'2026-10-09T08:00:00Z' }",
        )
        .await;

        assert!(
            tokio::time::timeout(Duration::from_millis(500), avisos.notified())
                .await
                .is_err(),
            "lo de otra unidad no lo despierta"
        );
    }
}
