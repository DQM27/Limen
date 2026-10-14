//! El dominio para JavaScript. Cada función es una puerta a una regla de
//! `limen-dominio`: aquí no se decide nada. El Worker del panel carga esto y sólo
//! orquesta (guardar, firmar, responder).
//!
//! Las fechas viajan como milisegundos desde 1970 (lo que da `Date.now()`). Los errores
//! llegan a JavaScript como el código de la regla (`codigo_usado`, `codigo_vencido`…).
//!
//! Cada regla vive en una función normal (`regla_*`, que se prueba aquí) y la exportada
//! sólo convierte el error: `JsValue` no existe fuera de WebAssembly.

use chrono::{DateTime, Utc};
use limen_dominio::auditoria::CambioCampo;
use limen_dominio::nombre::{NombrePersona, nombre_mientras_se_escribe};
use limen_dominio::operador::OperadorId;
use limen_dominio::usuario::{
    ClaveNueva, ErrorUsuario, HashClave, HechosUsuario, LARGO_CLAVE_TEMPORAL, Usuario,
    UsuarioGuardado, cedula_de_usuario, validar_datos,
};
use limen_dominio::vinculacion::{
    CodigoVinculacion, EmisionDeCodigo, ErrorVinculacion, LARGO_CODIGO, VIGENCIA_CODIGO_MINUTOS,
    puede_conectarse,
};
use serde_json::json;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

type Motivo = &'static str;

fn a_js(motivo: Motivo) -> JsValue {
    JsValue::from_str(motivo)
}

fn instante(milisegundos: f64) -> Result<DateTime<Utc>, Motivo> {
    // Las fechas llegan de `Date.now()`: enteras y dentro de lo que cabe en un `i64`.
    #[allow(clippy::cast_possible_truncation)]
    DateTime::from_timestamp_millis(milisegundos as i64).ok_or("fecha_invalida")
}

fn regla_generar(azar: &[u8]) -> Result<String, Motivo> {
    let azar: [u8; LARGO_CODIGO] = azar.try_into().map_err(|_| "azar_invalido")?;
    Ok(CodigoVinculacion::desde_azar(azar).as_str().to_owned())
}

fn regla_leer(texto: &str) -> Result<String, Motivo> {
    CodigoVinculacion::leer(texto)
        .map(|codigo| codigo.as_str().to_owned())
        .map_err(ErrorVinculacion::codigo)
}

fn regla_para_mostrar(texto: &str) -> Result<String, Motivo> {
    CodigoVinculacion::leer(texto)
        .map(|codigo| codigo.para_mostrar())
        .map_err(ErrorVinculacion::codigo)
}

fn regla_motivo_de_rechazo(
    emitido_ms: f64,
    usado: bool,
    anulado: bool,
    ahora_ms: f64,
) -> Result<String, Motivo> {
    let emision = EmisionDeCodigo {
        emitido_en: instante(emitido_ms)?,
        usado,
        anulado,
    };
    Ok(emision
        .puede_canjearse(instante(ahora_ms)?)
        .err()
        .map(ErrorVinculacion::codigo)
        .unwrap_or_default()
        .to_owned())
}

fn regla_motivo_de_conexion(vinculado: bool, retirado: bool) -> String {
    puede_conectarse(vinculado, retirado)
        .err()
        .map(ErrorVinculacion::codigo)
        .unwrap_or_default()
        .to_owned()
}

/// Un código nuevo con el azar que entrega el Worker (`crypto.getRandomValues`, 10 bytes).
#[wasm_bindgen]
pub fn generar_codigo(azar: &[u8]) -> Result<String, JsValue> {
    regla_generar(azar).map_err(a_js)
}

/// Lee lo que escribió o escaneó una persona y lo deja en su forma canónica (lo que se
/// transforma en huella).
#[wasm_bindgen]
pub fn leer_codigo(texto: &str) -> Result<String, JsValue> {
    regla_leer(texto).map_err(a_js)
}

/// El código como se le muestra a la persona (`XXXXX-XXXXX`).
#[wasm_bindgen]
pub fn codigo_para_mostrar(texto: &str) -> Result<String, JsValue> {
    regla_para_mostrar(texto).map_err(a_js)
}

/// ¿Se puede canjear un código emitido en `emitido_ms`? Vacío si sí; si no, el código
/// de la regla que lo impide.
#[wasm_bindgen]
pub fn motivo_de_rechazo(
    emitido_ms: f64,
    usado: bool,
    anulado: bool,
    ahora_ms: f64,
) -> Result<String, JsValue> {
    regla_motivo_de_rechazo(emitido_ms, usado, anulado, ahora_ms).map_err(a_js)
}

/// ¿Puede conectarse un equipo con ese estado? Vacío si sí; si no, el código de la regla.
#[wasm_bindgen]
pub fn motivo_de_rechazo_del_equipo(vinculado: bool, retirado: bool) -> String {
    regla_motivo_de_conexion(vinculado, retirado)
}

/// Cuántos minutos vale un código.
#[wasm_bindgen]
pub fn vigencia_minutos() -> f64 {
    f64::from(u32::try_from(VIGENCIA_CODIGO_MINUTOS).unwrap_or(u32::MAX))
}

/// Los datos de un usuario ya en su forma única, tal como el núcleo los decide: la pantalla
/// no normaliza nada, sólo muestra esto.
#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatosDeUsuario {
    pub cedula: String,
    pub nombre: String,
}

fn regla_datos_de_usuario(
    cedula: &str,
    nombre: &str,
    cedula_en_uso: bool,
) -> Result<DatosDeUsuario, Motivo> {
    validar_datos(cedula, nombre, cedula_en_uso)
        .map(|datos| DatosDeUsuario {
            cedula: datos.cedula.as_str().to_owned(),
            nombre: datos.nombre.as_str().to_owned(),
        })
        .map_err(ErrorUsuario::codigo)
}

fn regla_clave_temporal(azar: &[u8], cedula: &str) -> Result<String, Motivo> {
    let azar: [u8; LARGO_CLAVE_TEMPORAL] = azar.try_into().map_err(|_| "azar_invalido")?;
    let cedula = cedula_de_usuario(cedula).map_err(ErrorUsuario::codigo)?;
    ClaveNueva::temporal(azar, &cedula)
        .map(|clave| clave.as_str().to_owned())
        .map_err(ErrorUsuario::codigo)
}

/// Quien hace el cambio desde el panel: todavía no hay un usuario con nombre (llega con
/// Cloudflare Access), así que se anota con el identificador nulo. Sólo importa para la
/// regla «nadie se desactiva a sí mismo», que a un administrador del panel no le aplica.
fn administrador_del_panel() -> OperadorId {
    OperadorId::desde_uuid(Uuid::nil())
}

/// La clave real la tiene el Worker (cifrada en la base); el núcleo sólo decide qué cambió.
fn sin_clave() -> HashClave {
    HashClave::desde_texto(String::new())
}

fn cambios_a_json(cambios: &[CambioCampo]) -> serde_json::Value {
    cambios
        .iter()
        .map(|c| json!({ "campo": c.campo, "antes": c.antes, "despues": c.despues }))
        .collect()
}

/// El alta de un usuario: sus datos normalizados y lo que va a la auditoría.
fn regla_usuario_para_alta(
    cedula: &str,
    nombre: &str,
    cedula_en_uso: bool,
) -> Result<String, Motivo> {
    let datos = validar_datos(cedula, nombre, cedula_en_uso).map_err(ErrorUsuario::codigo)?;
    let usuario = Usuario::registrar(
        administrador_del_panel(),
        datos.cedula,
        datos.nombre.as_str(),
        sin_clave(),
        true,
        HechosUsuario {
            cedula_en_uso: false,
        },
    )
    .map_err(ErrorUsuario::codigo)?;
    Ok(json!({
        "cedula": usuario.cedula().as_str(),
        "nombre": usuario.nombre().as_str(),
        "cambios": cambios_a_json(&usuario.cambios_de_alta()),
    })
    .to_string())
}

/// Como estaba un usuario antes de cambiarlo, tal cual está en la base.
fn usuario_guardado(
    id: &str,
    cedula: &str,
    nombre: &str,
    activo: bool,
    debe_cambiar_clave: bool,
) -> Result<Usuario, Motivo> {
    Ok(Usuario::restaurar(UsuarioGuardado {
        id: OperadorId::desde_uuid(Uuid::parse_str(id).map_err(|_| "id_invalido")?),
        cedula: cedula_de_usuario(cedula).map_err(ErrorUsuario::codigo)?,
        nombre: NombrePersona::nuevo(nombre).map_err(|_| "nombre_invalido")?,
        activo,
        clave: sin_clave(),
        debe_cambiar_clave,
    }))
}

/// La edición de un usuario (cédula, nombre y activo): lo nuevo normalizado y lo que cambió.
#[allow(clippy::too_many_arguments)] // cada dato viene suelto de JavaScript, sin objetos
fn regla_usuario_para_edicion(
    id: &str,
    antes_cedula: &str,
    antes_nombre: &str,
    antes_activo: bool,
    cedula: &str,
    nombre: &str,
    activo: bool,
    cedula_en_uso: bool,
) -> Result<String, Motivo> {
    let mut usuario = usuario_guardado(id, antes_cedula, antes_nombre, antes_activo, false)?;
    let cedula = cedula_de_usuario(cedula).map_err(ErrorUsuario::codigo)?;
    let cambios = usuario
        .editar(
            cedula,
            nombre,
            activo,
            administrador_del_panel(),
            HechosUsuario { cedula_en_uso },
        )
        .map_err(ErrorUsuario::codigo)?;
    Ok(json!({
        "cedula": usuario.cedula().as_str(),
        "nombre": usuario.nombre().as_str(),
        "activo": usuario.activo(),
        "cambios": cambios_a_json(&cambios),
    })
    .to_string())
}

/// Restablecer la clave de un usuario: lo que va a la auditoría (nunca la clave).
fn regla_usuario_para_restablecer(
    id: &str,
    cedula: &str,
    nombre: &str,
    activo: bool,
    debe_cambiar_clave: bool,
) -> Result<String, Motivo> {
    let mut usuario = usuario_guardado(id, cedula, nombre, activo, debe_cambiar_clave)?;
    let cambios = usuario.restablecer_clave(sin_clave());
    Ok(json!({ "cambios": cambios_a_json(&cambios) }).to_string())
}

/// El alta de un usuario. Devuelve un JSON `{ cedula, nombre, cambios }`; si algo no vale, el
/// error es el código de la regla.
#[wasm_bindgen]
pub fn usuario_para_alta(
    cedula: &str,
    nombre: &str,
    cedula_en_uso: bool,
) -> Result<String, JsValue> {
    regla_usuario_para_alta(cedula, nombre, cedula_en_uso).map_err(a_js)
}

/// La edición de un usuario. Devuelve un JSON `{ cedula, nombre, activo, cambios }`.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)] // cada dato viene suelto de JavaScript, sin objetos
pub fn usuario_para_edicion(
    id: &str,
    antes_cedula: &str,
    antes_nombre: &str,
    antes_activo: bool,
    cedula: &str,
    nombre: &str,
    activo: bool,
    cedula_en_uso: bool,
) -> Result<String, JsValue> {
    regla_usuario_para_edicion(
        id,
        antes_cedula,
        antes_nombre,
        antes_activo,
        cedula,
        nombre,
        activo,
        cedula_en_uso,
    )
    .map_err(a_js)
}

/// Restablecer la clave de un usuario. Devuelve un JSON `{ cambios }`.
#[wasm_bindgen]
pub fn usuario_para_restablecer(
    id: &str,
    cedula: &str,
    nombre: &str,
    activo: bool,
    debe_cambiar_clave: bool,
) -> Result<String, JsValue> {
    regla_usuario_para_restablecer(id, cedula, nombre, activo, debe_cambiar_clave).map_err(a_js)
}

/// Valida y normaliza la cédula y el nombre de un usuario (alta o edición). `cedula_en_uso`:
/// si **otro** usuario ya tiene esa cédula. Si algo no vale, el error es el código de la
/// regla (`cedula_invalida`, `nombre_vacio`, `usuario_cedula_repetida`…).
#[wasm_bindgen]
pub fn datos_de_usuario(
    cedula: &str,
    nombre: &str,
    cedula_en_uso: bool,
) -> Result<DatosDeUsuario, JsValue> {
    regla_datos_de_usuario(cedula, nombre, cedula_en_uso).map_err(a_js)
}

/// El nombre como se ve mientras se escribe (mayúscula, sin tocar los espacios).
#[wasm_bindgen]
pub fn nombre_al_escribir(texto: &str) -> String {
    nombre_mientras_se_escribe(texto)
}

/// Una clave temporal con el azar que entrega el Worker (`crypto.getRandomValues`, 10 bytes).
#[wasm_bindgen]
pub fn clave_temporal(azar: &[u8], cedula: &str) -> Result<String, JsValue> {
    regla_clave_temporal(azar, cedula).map_err(a_js)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_datos_de_un_usuario_salen_normalizados() {
        let datos = regla_datos_de_usuario("1-1111-1111", "  josé  peña ", false).unwrap();
        assert_eq!(datos.cedula, "111111111");
        assert_eq!(datos.nombre, "JOSE PEÑA");
    }

    #[test]
    fn lo_que_no_vale_llega_como_el_codigo_de_la_regla() {
        assert_eq!(
            regla_datos_de_usuario("", "ana", false),
            Err("cedula_vacia")
        );
        assert_eq!(
            regla_datos_de_usuario("AB123", "ana", false),
            Err("cedula_invalida")
        );
        assert_eq!(
            regla_datos_de_usuario("1-1111-1111", "", false),
            Err("nombre_vacio")
        );
        assert_eq!(
            regla_datos_de_usuario("1-1111-1111", "ana", true),
            Err("usuario_cedula_repetida")
        );
    }

    const ID: &str = "0199c7a2-0000-7000-8000-000000000001";

    fn json_de(texto: &str) -> serde_json::Value {
        serde_json::from_str(texto).unwrap()
    }

    #[test]
    fn el_alta_devuelve_los_datos_normalizados_y_la_auditoria_sin_clave() {
        let alta = json_de(&regla_usuario_para_alta("1-1111-1111", " ana  mora ", false).unwrap());
        assert_eq!(alta["cedula"], "111111111");
        assert_eq!(alta["nombre"], "ANA MORA");
        let campos: Vec<&str> = alta["cambios"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["campo"].as_str().unwrap())
            .collect();
        assert_eq!(campos, ["cedula", "nombre", "activo", "debe_cambiar_clave"]);
        assert!(!alta.to_string().contains("clave\":"), "nunca la clave");
        assert_eq!(
            regla_usuario_para_alta("1-1111-1111", "ana", true),
            Err("usuario_cedula_repetida")
        );
    }

    #[test]
    fn la_edicion_cambia_cedula_y_nombre_y_dice_que_cambio() {
        let edicion = json_de(
            &regla_usuario_para_edicion(
                ID,
                "111111111",
                "ANA MORA",
                true,
                "2-2222-2222",
                "ana mora vega",
                true,
                false,
            )
            .unwrap(),
        );
        assert_eq!(edicion["cedula"], "222222222");
        assert_eq!(edicion["nombre"], "ANA MORA VEGA");
        assert_eq!(edicion["activo"], true);
        let cambios = edicion["cambios"].as_array().unwrap();
        assert_eq!(cambios.len(), 2, "{cambios:?}");
        assert_eq!(cambios[0]["campo"], "cedula");
        assert_eq!(cambios[0]["antes"], "111111111");
        assert_eq!(cambios[0]["despues"], "222222222");
    }

    #[test]
    fn la_edicion_respeta_la_cedula_unica_y_valida_lo_escrito() {
        let editar = |cedula: &str, nombre: &str, en_uso: bool| {
            regla_usuario_para_edicion(
                ID,
                "111111111",
                "ANA MORA",
                true,
                cedula,
                nombre,
                true,
                en_uso,
            )
        };
        assert_eq!(
            editar("2-2222-2222", "ana", true),
            Err("usuario_cedula_repetida")
        );
        assert!(
            editar("1-1111-1111", "ana", true).is_ok(),
            "su propia cédula no cuenta"
        );
        assert_eq!(editar("AB123", "ana", false), Err("cedula_invalida"));
        assert_eq!(editar("2-2222-2222", "", false), Err("nombre_vacio"));
    }

    #[test]
    fn restablecer_la_clave_la_deja_temporal_sin_decir_cual() {
        let reset = json_de(
            &regla_usuario_para_restablecer(ID, "111111111", "ANA MORA", true, false).unwrap(),
        );
        let cambios = reset["cambios"].as_array().unwrap();
        assert_eq!(cambios[0]["campo"], "clave");
        assert_eq!(cambios[0]["despues"], "cambiada");
        assert_eq!(cambios[1]["campo"], "debe_cambiar_clave");
        assert_eq!(cambios[1]["despues"], "true");
        assert_eq!(
            regla_usuario_para_restablecer("no-es-un-id", "111111111", "ANA", true, false),
            Err("id_invalido")
        );
    }

    #[test]
    fn el_nombre_al_escribir_no_toca_los_espacios() {
        assert_eq!(nombre_al_escribir("ana "), "ANA ");
    }

    #[test]
    fn la_clave_temporal_sale_del_azar_y_se_rechaza_si_no_es_de_diez() {
        assert_eq!(
            regla_clave_temporal(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 255], "1-1111-1111"),
            Ok("23456789AZ".to_owned())
        );
        assert_eq!(
            regla_clave_temporal(&[1, 2, 3], "1-1111-1111"),
            Err("azar_invalido")
        );
        assert_eq!(
            regla_clave_temporal(&[0; 10], "AB123"),
            Err("cedula_invalida")
        );
    }

    #[test]
    fn el_azar_de_otro_largo_se_rechaza() {
        assert_eq!(regla_generar(&[1, 2, 3]), Err("azar_invalido"));
    }

    #[test]
    fn el_codigo_generado_se_lee_y_se_muestra() {
        let codigo = regla_generar(&[0; 10]).unwrap();
        assert_eq!(regla_leer(&codigo), Ok(codigo.clone()));
        assert_eq!(regla_para_mostrar(&codigo).unwrap().len(), 11);
    }

    #[test]
    fn un_codigo_recien_emitido_no_tiene_motivo_de_rechazo() {
        assert_eq!(
            regla_motivo_de_rechazo(1_000.0, false, false, 2_000.0),
            Ok(String::new())
        );
    }

    #[test]
    fn el_motivo_es_el_codigo_de_la_regla() {
        let pasado_el_plazo = 1_000.0 + vigencia_minutos() * 60_000.0;
        assert_eq!(
            regla_motivo_de_rechazo(1_000.0, false, false, pasado_el_plazo),
            Ok("codigo_vencido".to_owned())
        );
        assert_eq!(
            regla_motivo_de_rechazo(1_000.0, true, false, 2_000.0),
            Ok("codigo_usado".to_owned())
        );
    }

    #[test]
    fn un_equipo_retirado_o_sin_vincular_no_se_conecta() {
        assert_eq!(regla_motivo_de_conexion(true, false), "");
        assert_eq!(regla_motivo_de_conexion(true, true), "equipo_retirado");
        assert_eq!(
            regla_motivo_de_conexion(false, false),
            "equipo_sin_vincular"
        );
    }
}
