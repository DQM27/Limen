//! Vincular este equipo con un código de un solo uso (bloque M, reglas M6 a M8).
//!
//! El equipo genera su par de claves P-256: **la privada no sale de él**. Manda al
//! Worker del panel el código y la pública; si el código vale (lo decide el dominio en el
//! Worker), responde con el ID del equipo y su unidad. Aquí se guarda lo que el equipo
//! necesita: su identidad (`equipo.txt`, que ya lee el resto de la app), su clave privada
//! (`clave-equipo.json`) y a dónde se vinculó (`vinculacion.json`).
//!
//! La clave privada queda por ahora en un archivo de la carpeta de datos de la app; con
//! DPAPI en Windows y Keystore en Android (`arquitectura.md`, 8.2) se cifrará en reposo.

use std::path::Path;

use limen_dominio::origen::{EquipoId, Origen, UnidadOperativaId};
use limen_dominio::vinculacion::{CodigoVinculacion, ErrorVinculacion};
use limen_infra_plataforma::guardar_origen;
use p256::SecretKey;
use p256::elliptic_curve::rand_core::OsRng;
use serde_json::{Value, json};
use uuid::Uuid;

/// A dónde se canjea el código si el equipo no dice otra cosa.
pub const PANEL_POR_OMISION: &str = "https://limen.megabrisas.com";
/// La variable de entorno que cambia el panel (pruebas, otro dominio).
pub const VAR_PANEL: &str = "LIMEN_PANEL_URL";

pub const ARCHIVO_CLAVE: &str = "clave-equipo.json";
pub const ARCHIVO_VINCULACION: &str = "vinculacion.json";

#[derive(Debug, thiserror::Error)]
pub enum ErrorDeVinculacion {
    /// El código no tiene la forma de un código (lo dice el dominio).
    #[error("{0}")]
    Codigo(ErrorVinculacion),
    /// No se pudo hablar con el panel.
    #[error("No se pudo comunicar con el panel: {0}")]
    Red(String),
    /// El panel rechazó el código; trae el código de la regla (`codigo_vencido`…).
    #[error("El panel rechazó el código: {0}")]
    Rechazado(String),
    #[error("La respuesta del panel no se entiende: {0}")]
    Respuesta(String),
    #[error("No se pudo guardar la vinculación: {0}")]
    Guardar(String),
}

impl ErrorDeVinculacion {
    /// El código de error para la interfaz (reglas del dominio cuando las hay).
    pub fn codigo(&self) -> &str {
        match self {
            Self::Codigo(e) => e.codigo(),
            Self::Rechazado(motivo) => motivo,
            Self::Red(_) => "sin_conexion",
            Self::Respuesta(_) => "respuesta_invalida",
            Self::Guardar(_) => "no_se_pudo_guardar",
        }
    }
}

/// Lo que el panel devuelve cuando acepta el código.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipoVinculado {
    pub origen: Origen,
    pub huella: String,
}

/// El par de claves del equipo: la pública (JWK) va al panel y la privada se queda.
struct ParDeClaves {
    publica: Value,
    privada: String,
}

fn generar_par() -> Result<ParDeClaves, ErrorDeVinculacion> {
    let privada = SecretKey::random(&mut OsRng);
    let publica = serde_json::from_str(&privada.public_key().to_jwk_string())
        .map_err(|e| ErrorDeVinculacion::Respuesta(e.to_string()))?;
    Ok(ParDeClaves {
        publica,
        privada: privada.to_jwk_string().to_string(),
    })
}

/// Lo que el panel respondió, ya interpretado.
fn interpretar(estado: u16, cuerpo: &Value) -> Result<EquipoVinculado, ErrorDeVinculacion> {
    if estado != 200 {
        let motivo = cuerpo
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("rechazado");
        return Err(ErrorDeVinculacion::Rechazado(motivo.to_owned()));
    }
    let texto = |clave: &str| {
        cuerpo
            .get(clave)
            .and_then(Value::as_str)
            .ok_or_else(|| ErrorDeVinculacion::Respuesta(format!("falta `{clave}`")))
    };
    let equipo = Uuid::parse_str(texto("equipo")?)
        .map_err(|_| ErrorDeVinculacion::Respuesta("`equipo` no es un identificador".to_owned()))?;
    let unidad = UnidadOperativaId::nueva(texto("unidad")?)
        .map_err(|e| ErrorDeVinculacion::Respuesta(e.to_string()))?;
    Ok(EquipoVinculado {
        origen: Origen {
            unidad,
            equipo: EquipoId::desde_uuid(equipo),
        },
        huella: texto("huella")?.to_owned(),
    })
}

/// ¿Este equipo ya se vinculó con un código? (Existe su clave y a dónde se vinculó.)
pub fn esta_vinculado(carpeta: &Path) -> bool {
    carpeta.join(ARCHIVO_CLAVE).is_file() && carpeta.join(ARCHIVO_VINCULACION).is_file()
}

/// Vincula este equipo: genera su par de claves, canjea el código en el panel y guarda su
/// identidad en `carpeta` (la de datos de la app). Tras vincularse hay que reiniciar la
/// app: la base se abrió con la identidad anterior.
pub async fn vincular(
    carpeta: &Path,
    panel: &str,
    codigo: &str,
) -> Result<EquipoVinculado, ErrorDeVinculacion> {
    let codigo = CodigoVinculacion::leer(codigo).map_err(ErrorDeVinculacion::Codigo)?;
    let par = generar_par()?;
    let respuesta = reqwest::Client::new()
        .post(format!("{}/api/vincular", panel.trim_end_matches('/')))
        .json(&json!({ "codigo": codigo.as_str(), "clave_publica": par.publica }))
        .send()
        .await
        .map_err(|e| ErrorDeVinculacion::Red(e.to_string()))?;
    let estado = respuesta.status().as_u16();
    let cuerpo: Value = respuesta
        .json()
        .await
        .map_err(|e| ErrorDeVinculacion::Respuesta(e.to_string()))?;
    let vinculado = interpretar(estado, &cuerpo)?;
    guardar(carpeta, panel, &par, &vinculado)?;
    Ok(vinculado)
}

fn guardar(
    carpeta: &Path,
    panel: &str,
    par: &ParDeClaves,
    vinculado: &EquipoVinculado,
) -> Result<(), ErrorDeVinculacion> {
    let falla = ErrorDeVinculacion::Guardar;
    std::fs::create_dir_all(carpeta).map_err(|e| falla(e.to_string()))?;
    std::fs::write(carpeta.join(ARCHIVO_CLAVE), &par.privada).map_err(|e| falla(e.to_string()))?;
    let datos = json!({ "panel": panel, "huella": vinculado.huella });
    std::fs::write(carpeta.join(ARCHIVO_VINCULACION), datos.to_string())
        .map_err(|e| falla(e.to_string()))?;
    guardar_origen(&carpeta.join("equipo.txt"), &vinculado.origen).map_err(falla)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EQUIPO: &str = "0199c7a2-0000-7000-8000-000000000001";

    #[test]
    fn el_par_de_claves_es_p256_y_la_publica_no_lleva_la_privada() {
        let par = generar_par().unwrap();
        assert_eq!(par.publica["kty"], "EC");
        assert_eq!(par.publica["crv"], "P-256");
        assert!(
            par.publica.get("d").is_none(),
            "la privada no sale del equipo"
        );
        let privada: Value = serde_json::from_str(&par.privada).unwrap();
        assert!(
            privada.get("d").is_some(),
            "la privada guarda su componente d"
        );
    }

    #[test]
    fn cada_equipo_genera_un_par_distinto() {
        assert_ne!(
            generar_par().unwrap().publica,
            generar_par().unwrap().publica
        );
    }

    #[test]
    fn una_respuesta_buena_trae_el_equipo_la_unidad_y_la_huella() {
        let respuesta = json!({ "equipo": EQUIPO, "unidad": "brisas", "huella": "abc" });
        let vinculado = interpretar(200, &respuesta).unwrap();
        assert_eq!(vinculado.origen.unidad.as_str(), "BRISAS");
        assert_eq!(vinculado.origen.equipo.to_string(), EQUIPO);
        assert_eq!(vinculado.huella, "abc");
    }

    #[test]
    fn un_rechazo_conserva_el_codigo_de_la_regla() {
        let error = interpretar(400, &json!({ "error": "codigo_vencido" })).unwrap_err();
        assert_eq!(error.codigo(), "codigo_vencido");
    }

    #[test]
    fn una_respuesta_incompleta_no_se_acepta() {
        let error = interpretar(200, &json!({ "equipo": EQUIPO })).unwrap_err();
        assert_eq!(error.codigo(), "respuesta_invalida");
    }

    #[tokio::test]
    async fn un_codigo_mal_escrito_ni_sale_a_la_red() {
        let carpeta = std::env::temp_dir().join(format!("limen-vinc-{}", Uuid::now_v7()));
        let error = vincular(&carpeta, "http://127.0.0.1:1", "ABC")
            .await
            .unwrap_err();
        assert_eq!(error.codigo(), "codigo_invalido");
        assert!(!esta_vinculado(&carpeta), "no quedó nada guardado");
    }

    #[test]
    fn guardar_deja_la_identidad_la_clave_y_el_panel() {
        let carpeta = std::env::temp_dir().join(format!("limen-vinc-{}", Uuid::now_v7()));
        let par = generar_par().unwrap();
        let vinculado = interpretar(
            200,
            &json!({ "equipo": EQUIPO, "unidad": "BRISAS", "huella": "abc" }),
        )
        .unwrap();

        guardar(&carpeta, "https://panel.prueba", &par, &vinculado).unwrap();

        assert!(esta_vinculado(&carpeta));
        let identidad = std::fs::read_to_string(carpeta.join("equipo.txt")).unwrap();
        assert!(identidad.contains(EQUIPO), "{identidad}");
        drop(std::fs::remove_dir_all(carpeta));
    }
}
