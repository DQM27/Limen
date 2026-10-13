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
use limen_dominio::vinculacion::{
    CodigoVinculacion, EmisionDeCodigo, ErrorVinculacion, LARGO_CODIGO, VIGENCIA_CODIGO_MINUTOS,
};
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

/// Cuántos minutos vale un código.
#[wasm_bindgen]
pub fn vigencia_minutos() -> f64 {
    f64::from(u32::try_from(VIGENCIA_CODIGO_MINUTOS).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
