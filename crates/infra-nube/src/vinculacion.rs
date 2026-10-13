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

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use limen_dominio::origen::{EquipoId, Origen, UnidadOperativaId};
use limen_dominio::vinculacion::{CodigoVinculacion, ErrorVinculacion};
use limen_infra_plataforma::{guardar_origen, origen_del_equipo};
use p256::SecretKey;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::rand_core::OsRng;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{ConfigNube, Credenciales};

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

/// Lo que el Worker responde al pedir el acceso, ya interpretado: la dirección de la nube
/// y el token con que este equipo entra.
fn acceso_desde(estado: u16, cuerpo: &Value) -> Result<ConfigNube, ErrorDeVinculacion> {
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
    Ok(ConfigNube {
        url: texto("url")?.to_owned(),
        credenciales: Some(Credenciales::con_token(texto("token")?.to_owned())),
    })
}

/// La firma del equipo sobre un mensaje: ECDSA P-256 con SHA-256, en el formato de
/// `WebCrypto` (r y s juntos, 64 bytes) y en base64 sin relleno.
fn firmar(secreta: &SecretKey, mensaje: &str) -> String {
    let firma: Signature = SigningKey::from(secreta).sign(mensaje.as_bytes());
    URL_SAFE_NO_PAD.encode(firma.to_bytes())
}

async fn enviar(
    cliente: &reqwest::Client,
    url: &str,
    cuerpo: &Value,
) -> Result<(u16, Value), ErrorDeVinculacion> {
    let respuesta = cliente
        .post(url)
        .json(cuerpo)
        .send()
        .await
        .map_err(|e| ErrorDeVinculacion::Red(e.to_string()))?;
    let estado = respuesta.status().as_u16();
    let cuerpo = respuesta
        .json()
        .await
        .map_err(|e| ErrorDeVinculacion::Respuesta(e.to_string()))?;
    Ok((estado, cuerpo))
}

/// Pide al Worker del panel el acceso de este equipo a la nube. El equipo prueba quién es
/// firmando un desafío con su clave privada (que nunca sale de él); si sigue vinculado y
/// no lo han retirado, recibe un token. Se pide en cada conexión: un equipo retirado no
/// vuelve a entrar.
pub async fn pedir_acceso(carpeta: &Path) -> Result<ConfigNube, ErrorDeVinculacion> {
    let falla = ErrorDeVinculacion::Guardar;
    let leer = |nombre: &str| {
        std::fs::read_to_string(carpeta.join(nombre)).map_err(|e| falla(format!("{nombre}: {e}")))
    };
    let origen = origen_del_equipo(&carpeta.join("equipo.txt"), "BRISAS").map_err(falla)?;
    let secreta = SecretKey::from_jwk_str(&leer(ARCHIVO_CLAVE)?)
        .map_err(|e| ErrorDeVinculacion::Guardar(format!("{ARCHIVO_CLAVE}: {e}")))?;
    let datos: Value = serde_json::from_str(&leer(ARCHIVO_VINCULACION)?)
        .map_err(|e| ErrorDeVinculacion::Guardar(format!("{ARCHIVO_VINCULACION}: {e}")))?;
    let panel = datos
        .get("panel")
        .and_then(Value::as_str)
        .ok_or_else(|| ErrorDeVinculacion::Guardar("falta `panel`".to_owned()))?;
    let url = format!("{}/api/token", panel.trim_end_matches('/'));
    let equipo = origen.equipo.to_string();
    let cliente = reqwest::Client::new();

    let (estado, cuerpo) = enviar(&cliente, &url, &json!({ "equipo": equipo })).await?;
    let desafio = match (estado, cuerpo.get("desafio").and_then(Value::as_str)) {
        (200, Some(desafio)) => desafio.to_owned(),
        _ => {
            return Err(acceso_desde(estado, &cuerpo)
                .err()
                .unwrap_or_else(|| ErrorDeVinculacion::Respuesta("falta `desafio`".to_owned())));
        }
    };
    let firma = firmar(&secreta, &desafio);
    let (estado, cuerpo) = enviar(
        &cliente,
        &url,
        &json!({ "equipo": equipo, "desafio": desafio, "firma": firma }),
    )
    .await?;
    acceso_desde(estado, &cuerpo)
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

    #[test]
    fn la_firma_es_de_64_bytes_y_se_verifica_con_la_publica() {
        use p256::ecdsa::VerifyingKey;
        use p256::ecdsa::signature::Verifier;

        let secreta = SecretKey::random(&mut OsRng);
        let firma = firmar(&secreta, "desafio-de-prueba");

        let bytes = URL_SAFE_NO_PAD.decode(&firma).unwrap();
        assert_eq!(bytes.len(), 64, "r y s juntos, como WebCrypto");
        let firma = Signature::from_slice(&bytes).unwrap();
        let publica = VerifyingKey::from(&SigningKey::from(&secreta));
        assert!(publica.verify(b"desafio-de-prueba", &firma).is_ok());
        assert!(
            publica.verify(b"otro-desafio", &firma).is_err(),
            "la firma sólo vale para ese desafío"
        );
    }

    #[test]
    fn el_acceso_trae_la_url_de_la_nube_y_el_token() {
        let respuesta = json!({ "url": "wss://nube.prueba", "token": "abc.def" });

        let config = acceso_desde(200, &respuesta).unwrap();

        assert_eq!(config.url, "wss://nube.prueba");
        let credenciales = config.credenciales.unwrap();
        assert_eq!(credenciales.nivel, crate::Nivel::Token);
        assert_eq!(credenciales.clave, "abc.def");
    }

    #[test]
    fn un_equipo_retirado_recibe_el_motivo_y_ningun_acceso() {
        let error = acceso_desde(401, &json!({ "error": "dispositivo_invalido" })).unwrap_err();
        assert_eq!(error.codigo(), "dispositivo_invalido");
    }
}
