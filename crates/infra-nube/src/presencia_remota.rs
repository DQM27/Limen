//! Regla A9: preguntar a la nube si una persona ya está adentro en otra unidad.
//!
//! Un equipo sólo ve en la base lo de su unidad, así que la pregunta va al Worker del panel
//! (`GET /api/presencia/:clave`), con el token que el Worker le dio al equipo. Este adaptador
//! no decide nada: trae lo que la nube sabe. Si no hay nube, no hay token, la red falla o la
//! respuesta tarda más de [`ESPERA`], responde `None`: el acceso nunca se bloquea por la red y
//! el choque, si lo hay, se marca al sincronizar (`docs/operacion-sin-conexion.md`).

use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::PresenciaRemota;
use limen_dominio::origen::UnidadOperativaId;
use limen_dominio::presencia::{AdentroEnOtraUnidad, Identidad, Via};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::vinculacion::ARCHIVO_VINCULACION;
use crate::{FuenteDeConfig, Nivel};

/// Cuánto se espera la respuesta antes de tratarla como «sin conexión».
pub const ESPERA: Duration = Duration::from_secs(3);
/// El token del Worker dura una hora: se renueva un poco antes.
const VIGENCIA_DEL_TOKEN: Duration = Duration::from_secs(50 * 60);

struct Conexion {
    panel: String,
    fuente: FuenteDeConfig,
    http: reqwest::Client,
    token: Mutex<Option<(String, Instant)>>,
}

/// Se crea vacía al armar la aplicación y se conecta cuando se sabe si el equipo está
/// vinculado: hasta entonces (o si no lo está) no hay nube y responde `None`.
#[derive(Debug, Clone, Default)]
pub struct PresenciaNube {
    conexion: Arc<OnceLock<Conexion>>,
}

impl PresenciaNube {
    /// Conecta con el panel al que se vinculó este equipo (`vinculacion.json`). Devuelve
    /// `false`, sin hacer nada, si no se puede leer a dónde.
    pub fn conectar(&self, carpeta: &Path, fuente: FuenteDeConfig) -> bool {
        let Some(panel) = panel_de(carpeta) else {
            return false;
        };
        self.conexion
            .set(Conexion {
                panel,
                fuente,
                http: reqwest::Client::new(),
                token: Mutex::new(None),
            })
            .is_ok()
    }

    async fn preguntar(&self, identidad: &Identidad) -> Option<AdentroEnOtraUnidad> {
        let conexion = self.conexion.get()?;
        let token = conexion.token().await?;
        let url = format!(
            "{}/api/presencia/{}",
            conexion.panel.trim_end_matches('/'),
            identidad.clave()
        );
        let respuesta = conexion
            .http
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .inspect_err(|e| log::warn!("A9: no se pudo preguntar a la nube: {e}"))
            .ok()?;
        if !respuesta.status().is_success() {
            // Un token rechazado no se vuelve a usar: la próxima vez se pide otro.
            *conexion.token.lock().await = None;
            log::warn!("A9: el panel respondió {}", respuesta.status());
            return None;
        }
        interpretar(&respuesta.json().await.ok()?)
    }
}

impl std::fmt::Debug for Conexion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Sin la fuente de configuración ni el token: no deben llegar a ningún registro.
        f.debug_struct("Conexion")
            .field("panel", &self.panel)
            .finish_non_exhaustive()
    }
}

impl Conexion {
    /// El token vigente, o uno nuevo si venció. Sólo sirve el que el Worker emite a un equipo
    /// vinculado: con las variables `LIMEN_NUBE_*` (desarrollo) no hay a quién preguntar.
    async fn token(&self) -> Option<String> {
        let mut guardado = self.token.lock().await;
        if let Some((token, desde)) = guardado.as_ref()
            && desde.elapsed() < VIGENCIA_DEL_TOKEN
        {
            return Some(token.clone());
        }
        let config = (self.fuente)()
            .await
            .inspect_err(|e| log::warn!("A9: sin acceso a la nube: {e}"))
            .ok()?;
        let credenciales = config.credenciales?;
        if credenciales.nivel != Nivel::Token {
            return None;
        }
        *guardado = Some((credenciales.clave.clone(), Instant::now()));
        drop(guardado);
        Some(credenciales.clave)
    }
}

impl PresenciaRemota for PresenciaNube {
    async fn consultar(&self, identidad: &Identidad) -> Option<AdentroEnOtraUnidad> {
        tokio::time::timeout(ESPERA, self.preguntar(identidad))
            .await
            .inspect_err(|_| log::warn!("A9: la nube no respondió en {ESPERA:?}"))
            .ok()
            .flatten()
    }
}

fn panel_de(carpeta: &Path) -> Option<String> {
    let texto = std::fs::read_to_string(carpeta.join(ARCHIVO_VINCULACION)).ok()?;
    let datos: Value = serde_json::from_str(&texto).ok()?;
    datos.get("panel")?.as_str().map(str::to_owned)
}

/// `{ "adentro": null }` o `{ "adentro": { "unidad", "via", "desde" } }`. Lo que no se entienda
/// se trata como «no está adentro»: ante la duda no se bloquea el acceso.
fn interpretar(cuerpo: &Value) -> Option<AdentroEnOtraUnidad> {
    let adentro = cuerpo.get("adentro")?;
    let unidad = UnidadOperativaId::nueva(adentro.get("unidad")?.as_str()?).ok()?;
    let via = Via::desde_codigo(adentro.get("via")?.as_str()?)?;
    let desde = DateTime::parse_from_rfc3339(adentro.get("desde")?.as_str()?)
        .ok()?
        .with_timezone(&Utc);
    Some(AdentroEnOtraUnidad { unidad, via, desde })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn la_respuesta_con_alguien_adentro_se_entiende() {
        let respuesta = json!({ "adentro": {
            "unidad": "CARTAGO", "via": "PROVEEDOR", "desde": "2026-10-14T14:12:00.123456Z"
        }});
        let otra = interpretar(&respuesta).expect("está adentro");
        assert_eq!(otra.unidad.as_str(), "CARTAGO");
        assert_eq!(otra.via, Via::Proveedor);
        assert_eq!(otra.desde.to_rfc3339(), "2026-10-14T14:12:00.123456+00:00");
    }

    #[test]
    fn sin_nadie_adentro_o_con_basura_no_se_bloquea() {
        assert_eq!(interpretar(&json!({ "adentro": null })), None);
        assert_eq!(interpretar(&json!({})), None);
        assert_eq!(interpretar(&json!({ "error": "nube" })), None);
        let via_rara = json!({ "adentro": { "unidad": "CARTAGO", "via": "X", "desde": "2026-10-14T14:12:00Z" }});
        assert_eq!(interpretar(&via_rara), None);
        let sin_fecha = json!({ "adentro": { "unidad": "CARTAGO", "via": "KOF", "desde": "ayer" }});
        assert_eq!(interpretar(&sin_fecha), None);
    }

    #[tokio::test]
    async fn sin_conectar_no_hay_nube_y_responde_none() {
        let cedula = limen_dominio::cedula::Cedula::normalizar("1-1111-1111").unwrap();
        let nube = PresenciaNube::default();
        assert_eq!(nube.consultar(&Identidad::from(&cedula)).await, None);
    }

    #[test]
    fn sin_vinculacion_no_se_conecta() {
        let carpeta = std::env::temp_dir().join("limen-sin-vinculacion-a9");
        let fuente = crate::config_fija(crate::ConfigNube {
            url: "mem://".into(),
            credenciales: None,
        });
        assert!(!PresenciaNube::default().conectar(&carpeta, fuente));
    }
}
