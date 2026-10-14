//! El dominio para JavaScript. Cada función es una puerta a una regla de
//! `limen-dominio`: aquí no se decide nada. El Worker del panel carga esto y sólo
//! orquesta (guardar, firmar, responder).
//!
//! Las fechas viajan como milisegundos desde 1970 (lo que da `Date.now()`). Los errores
//! llegan a JavaScript como el código de la regla (`codigo_usado`, `codigo_vencido`…).
//!
//! Cada regla vive en una función normal (`regla_*`, que se prueba aquí) y la exportada
//! sólo convierte el error: `JsValue` no existe fuera de WebAssembly.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use limen_dominio::auditoria::CambioCampo;
use limen_dominio::busqueda::{buscador_mientras_se_escribe, plegar};
use limen_dominio::cedula::Cedula;
use limen_dominio::conexion_equipo::estado_de_conexion;
use limen_dominio::contratista::{
    BorradorContratista, Contratista, ContratistaGuardado, ContratistaId, DatosContratista,
    ErrorContratista, HechosContratista, cedula_de_contratista, problemas, tiene_praind,
};
use limen_dominio::empresa::{
    Empresa, EmpresaId, HechosEmpresa, NombreEmpresa, nombre_empresa_mientras_se_escribe,
};
use limen_dominio::filtro_de_tabla::{
    CampoDeFila, Operacion, cumple_filtro_de_columna, fila_coincide,
};
use limen_dominio::nombre::{NombrePersona, nombre_mientras_se_escribe};
use limen_dominio::operador::OperadorId;
use limen_dominio::praind::praind_vencido;
use limen_dominio::resaltado::{Campo, resaltar};
use limen_dominio::tipo_ingreso::TipoIngreso;
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

fn regla_estado_de_conexion(
    anuncia_en_linea: bool,
    visto_ms: f64,
    ahora_ms: f64,
) -> Result<String, Motivo> {
    Ok(
        estado_de_conexion(anuncia_en_linea, instante(visto_ms)?, instante(ahora_ms)?)
            .codigo()
            .to_owned(),
    )
}

/// Cómo está conectado un equipo: `en_linea`, `caido` (sin señal) o `fuera` (desconectado). Lo
/// decide el núcleo; el Worker y la página sólo muestran el código.
#[wasm_bindgen]
pub fn estado_de_conexion_del_equipo(
    anuncia_en_linea: bool,
    visto_ms: f64,
    ahora_ms: f64,
) -> Result<String, JsValue> {
    regla_estado_de_conexion(anuncia_en_linea, visto_ms, ahora_ms).map_err(a_js)
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

/// Las reglas de búsqueda de las tablas (N1 y N2): la pantalla no compara, no pliega ni parte
/// por palabras; pregunta y aplica lo que responde. Las formas son las mismas que usa la app
/// de escritorio. Los pedidos y las respuestas viajan como JSON.
fn leer_json(texto: &str) -> Result<serde_json::Value, Motivo> {
    serde_json::from_str(texto).map_err(|_| "json_invalido")
}

fn texto_de<'a>(valor: &'a serde_json::Value, clave: &str) -> Result<&'a str, Motivo> {
    valor
        .get(clave)
        .and_then(serde_json::Value::as_str)
        .ok_or("json_invalido")
}

fn lista_de(valor: &serde_json::Value) -> Result<&Vec<serde_json::Value>, Motivo> {
    valor.as_array().ok_or("json_invalido")
}

fn campo_resaltable(codigo: &str) -> Result<Campo, Motivo> {
    Campo::desde_codigo(codigo).ok_or("campo_invalido")
}

/// Cada texto partido en segmentos, con lo que coincide resaltado (regla N1).
fn regla_resaltar(buscado: &str, campo: &str, textos: &str) -> Result<String, Motivo> {
    let campo = campo_resaltable(campo)?;
    let textos: Vec<String> = lista_de(&leer_json(textos)?)?
        .iter()
        .map(|t| t.as_str().map(str::to_owned).ok_or("json_invalido"))
        .collect::<Result<_, _>>()?;
    let segmentos: Vec<Vec<serde_json::Value>> = resaltar(buscado, campo, &textos)
        .iter()
        .map(|texto| {
            texto
                .iter()
                .map(|s| json!({ "texto": s.texto, "resaltado": s.resaltado }))
                .collect()
        })
        .collect();
    Ok(json!(segmentos).to_string())
}

/// De cada fila, si coincide con lo escrito en el buscador de la tabla (regla N2).
fn regla_filas_que_coinciden(buscado: &str, filas: &str) -> Result<String, Motivo> {
    let filas = leer_json(filas)?;
    let respuestas: Vec<bool> = lista_de(&filas)?
        .iter()
        .map(|fila| {
            let campos: Vec<CampoDeFila<'_>> = lista_de(fila)?
                .iter()
                .map(|c| {
                    Ok(CampoDeFila {
                        campo: campo_resaltable(texto_de(c, "campo")?)?,
                        texto: texto_de(c, "texto")?,
                    })
                })
                .collect::<Result<_, Motivo>>()?;
            Ok(fila_coincide(buscado, &campos))
        })
        .collect::<Result<_, Motivo>>()?;
    Ok(json!(respuestas).to_string())
}

/// De cada celda, si cumple el filtro de columna que se le pide (regla N2).
fn regla_celdas_que_cumplen(pedidos: &str) -> Result<String, Motivo> {
    let pedidos = leer_json(pedidos)?;
    let respuestas: Vec<bool> = lista_de(&pedidos)?
        .iter()
        .map(|p| {
            let operacion =
                Operacion::desde_codigo(texto_de(p, "operacion")?).ok_or("operacion_invalida")?;
            Ok(cumple_filtro_de_columna(
                operacion,
                texto_de(p, "celda")?,
                texto_de(p, "filtro")?,
            ))
        })
        .collect::<Result<_, Motivo>>()?;
    Ok(json!(respuestas).to_string())
}

/// Parte cada texto en segmentos (`[[{ texto, resaltado }]]`). `campo`: `NOMBRE`,
/// `IDENTIFICACION` o `EMPRESA`; `textos`: un arreglo JSON de textos.
#[wasm_bindgen]
pub fn resaltar_coincidencias(buscado: &str, campo: &str, textos: &str) -> Result<String, JsValue> {
    regla_resaltar(buscado, campo, textos).map_err(a_js)
}

/// De cada fila (un arreglo de `{ campo, texto }`), si coincide con el buscador.
#[wasm_bindgen]
pub fn filas_que_coinciden(buscado: &str, filas: &str) -> Result<String, JsValue> {
    regla_filas_que_coinciden(buscado, filas).map_err(a_js)
}

/// De cada pedido (`{ operacion, celda, filtro }`), si la celda cumple el filtro de columna.
#[wasm_bindgen]
pub fn celdas_que_cumplen(pedidos: &str) -> Result<String, JsValue> {
    regla_celdas_que_cumplen(pedidos).map_err(a_js)
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

fn regla_mensaje_de_usuario(codigo: &str) -> String {
    ErrorUsuario::desde_codigo(codigo)
        .map(|error| error.to_string())
        .unwrap_or_default()
}

/// El mensaje que el núcleo da a un error de usuario, por su código (vacío si no es de los
/// suyos). La pantalla recibe sólo el código y muestra esto.
#[wasm_bindgen]
pub fn mensaje_de_error_de_usuario(codigo: &str) -> String {
    regla_mensaje_de_usuario(codigo)
}

/// Lo escrito en el buscador de una tabla como se ve (mayúsculas, sin tildes; regla N3).
#[wasm_bindgen]
pub fn buscador_al_escribir(texto: &str) -> String {
    buscador_mientras_se_escribe(texto)
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

// --- Contratistas y sus empresas ---

/// El día de hoy en la instalación. Costa Rica va seis horas detrás de UTC todo el año.
fn hoy_de_costa_rica(ahora_ms: f64) -> Result<NaiveDate, Motivo> {
    Ok((instante(ahora_ms)? - Duration::hours(6)).date_naive())
}

fn texto_opcional<'a>(valor: &'a serde_json::Value, clave: &str) -> &'a str {
    valor
        .get(clave)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

fn booleano(valor: &serde_json::Value, clave: &str) -> bool {
    valor
        .get(clave)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or_default()
}

fn uuid_de(texto: &str) -> Result<Uuid, Motivo> {
    Uuid::parse_str(texto).map_err(|_| "id_invalido")
}

/// La empresa elegida (su identificador), o ninguna si el formulario no trae.
fn empresa_elegida(valor: &serde_json::Value) -> Result<Option<EmpresaId>, Motivo> {
    match texto_opcional(valor, "empresa") {
        "" => Ok(None),
        texto => Ok(Some(EmpresaId::desde_uuid(uuid_de(texto)?))),
    }
}

/// La fecha escrita (`AAAA-MM-DD`), o ninguna si el formulario no trae.
fn fecha_escrita(valor: &serde_json::Value) -> Result<Option<NaiveDate>, Motivo> {
    match texto_opcional(valor, "fecha_vencimiento_praind") {
        "" => Ok(None),
        texto => texto.parse().map(Some).map_err(|_| "fecha_invalida"),
    }
}

fn tipo_de_ingreso(valor: &serde_json::Value) -> Result<TipoIngreso, Motivo> {
    TipoIngreso::desde_codigo(texto_opcional(valor, "tipo_ingreso")).map_err(|_| "tipo_invalido")
}

/// Como estaba un contratista antes de editarlo, tal cual está en la base.
fn contratista_anterior(valor: &serde_json::Value) -> Result<Contratista, Motivo> {
    Ok(Contratista::restaurar(ContratistaGuardado {
        id: ContratistaId::desde_uuid(uuid_de(texto_opcional(valor, "id"))?),
        cedula: Cedula::normalizar(texto_opcional(valor, "cedula"))
            .map_err(|_| "cedula_invalida")?,
        nombre: NombrePersona::nuevo(texto_opcional(valor, "nombre"))
            .map_err(|_| "nombre_invalido")?,
        empresa: empresa_elegida(valor)?.ok_or("id_invalido")?,
        tipo_ingreso: tipo_de_ingreso(valor)?,
        fecha_vencimiento_praind: fecha_escrita(valor)?.ok_or("fecha_invalida")?,
        tiene_acceso: booleano(valor, "tiene_acceso"),
    }))
}

/// El campo del formulario al que pertenece cada error (los mismos nombres que usa el
/// escritorio), para que la pantalla muestre el mensaje junto a él.
const fn campo_de_contratista(error: ErrorContratista) -> &'static str {
    match error {
        ErrorContratista::CedulaVacia
        | ErrorContratista::CedulaInvalida
        | ErrorContratista::CedulaRepetida
        | ErrorContratista::CedulaNoEditableAdentro => "cedula",
        ErrorContratista::NombreVacio | ErrorContratista::NombreInvalido => "nombre",
        ErrorContratista::EmpresaRequerida | ErrorContratista::EmpresaNoExiste => "empresa_id",
        ErrorContratista::PraindRequerido | ErrorContratista::PraindVencido => {
            "fecha_vencimiento_praind"
        }
    }
}

/// Un contratista (nuevo o `anterior`) ya evaluado por el núcleo: todos los problemas del
/// formulario a la vez o, si no hay ninguno, los datos normalizados y lo que cambió.
fn regla_contratista(entrada: &str) -> Result<String, Motivo> {
    let entrada = leer_json(entrada)?;
    let hoy = hoy_de_costa_rica(
        entrada
            .get("ahora_ms")
            .and_then(serde_json::Value::as_f64)
            .ok_or("json_invalido")?,
    )?;
    let anterior = match entrada.get("anterior").filter(|a| !a.is_null()) {
        Some(anterior) => Some(contratista_anterior(anterior)?),
        None => None,
    };
    let formulario = entrada.get("borrador").ok_or("json_invalido")?;
    let hechos_leidos = entrada.get("hechos").ok_or("json_invalido")?;
    let hechos = HechosContratista {
        cedula_en_uso: booleano(hechos_leidos, "cedula_en_uso"),
        empresa_existe: booleano(hechos_leidos, "empresa_existe"),
        esta_adentro: booleano(hechos_leidos, "esta_adentro"),
    };
    let (empresa, fecha) = (empresa_elegida(formulario)?, fecha_escrita(formulario)?);
    let borrador = BorradorContratista {
        cedula: texto_opcional(formulario, "cedula"),
        nombre: texto_opcional(formulario, "nombre"),
        empresa,
        fecha_vencimiento_praind: fecha,
        tiene_acceso: booleano(formulario, "tiene_acceso"),
    };
    let lista = problemas(&borrador, hechos, hoy, anterior.as_ref());
    if !lista.is_empty() {
        let lista: Vec<_> = lista
            .iter()
            .map(|error| {
                json!({
                    "codigo": error.codigo(),
                    "campo": campo_de_contratista(*error),
                    "mensaje": error.to_string(),
                })
            })
            .collect();
        return Ok(json!({ "problemas": lista }).to_string());
    }

    // Sin problemas, la empresa está (y la fecha, salvo que no se dé el acceso).
    let Some(empresa) = empresa else {
        return Err("json_invalido");
    };
    let datos = DatosContratista {
        cedula: borrador.cedula,
        nombre: borrador.nombre,
        empresa,
        tipo_ingreso: tipo_de_ingreso(formulario)?,
        fecha_vencimiento_praind: fecha,
        tiene_acceso: booleano(formulario, "tiene_acceso"),
    };
    let (contratista, cambios) = if let Some(mut contratista) = anterior {
        let cambios = contratista
            .editar(datos, hechos, hoy)
            .map_err(ErrorContratista::codigo)?;
        (contratista, cambios)
    } else {
        let id = ContratistaId::desde_uuid(uuid_de(texto_opcional(&entrada, "id"))?);
        let contratista =
            Contratista::registrar(id, datos, hechos, hoy).map_err(ErrorContratista::codigo)?;
        let cambios = contratista.cambios_de_alta();
        (contratista, cambios)
    };
    Ok(json!({
        "problemas": [],
        "datos": {
            "id": contratista.id().to_string(),
            "cedula": contratista.cedula().as_str(),
            "nombre": contratista.nombre().as_str(),
            "nombre_busqueda": plegar(contratista.nombre().as_str()),
            "empresa": contratista.empresa().to_string(),
            "tipo_ingreso": contratista.tipo_ingreso().codigo(),
            "fecha_vencimiento_praind": contratista.fecha_vencimiento_praind().to_string(),
            "tiene_acceso": contratista.tiene_acceso(),
        },
        "cambios": cambios_a_json(&cambios),
    })
    .to_string())
}

/// La cédula de un contratista en su forma única, o vacía si no vale (el formulario ya
/// informa el motivo): con ella el Worker averigua si otro contratista la tiene.
fn regla_cedula_de_contratista(texto: &str) -> String {
    cedula_de_contratista(texto)
        .map(|cedula| cedula.as_str().to_owned())
        .unwrap_or_default()
}

/// Una empresa (nueva, o con `anterior` si se renombra) evaluada por el núcleo: el problema
/// que impide guardarla o el nombre normalizado y lo que cambió.
fn regla_empresa(
    id: &str,
    anterior: &str,
    nombre: &str,
    nombre_en_uso: bool,
) -> Result<String, Motivo> {
    let hechos = HechosEmpresa { nombre_en_uso };
    let id = EmpresaId::desde_uuid(uuid_de(id)?);
    let resultado = if anterior.is_empty() {
        Empresa::registrar(id, nombre, hechos).map(|empresa| {
            let cambios = empresa.cambios_de_alta();
            (empresa, cambios)
        })
    } else {
        let anterior = NombreEmpresa::nuevo(anterior).map_err(|_| "nombre_invalido")?;
        let mut empresa = Empresa::restaurar(id, anterior);
        empresa
            .renombrar(nombre, hechos)
            .map(|cambios| (empresa, cambios))
    };
    Ok(match resultado {
        Ok((empresa, cambios)) => json!({
            "problema": null,
            "nombre": empresa.nombre().as_str(),
            "nombre_busqueda": plegar(empresa.nombre().as_str()),
            "cambios": cambios_a_json(&cambios),
        }),
        Err(error) => json!({
            "problema": {
                "codigo": error.codigo(),
                "campo": "nombre",
                "mensaje": error.to_string(),
            },
        }),
    }
    .to_string())
}

/// Evalúa el formulario de un contratista. Entrada JSON: `{ id, ahora_ms, anterior, borrador,
/// hechos }` (`anterior` es `null` en un alta). Salida JSON: `{ problemas }` con todo lo que
/// impide guardar, o `{ problemas: [], datos, cambios }` con lo que se guarda y se audita.
#[wasm_bindgen]
pub fn contratista_evaluar(entrada: &str) -> Result<String, JsValue> {
    regla_contratista(entrada).map_err(a_js)
}

/// La cédula del contratista en su forma única (vacía si no vale).
#[wasm_bindgen]
pub fn contratista_cedula(texto: &str) -> String {
    regla_cedula_de_contratista(texto)
}

/// Evalúa el nombre de una empresa (alta, o renombrar si `anterior` no está vacío). Devuelve
/// un JSON `{ problema }` o `{ problema: null, nombre, nombre_busqueda, cambios }`.
#[wasm_bindgen]
pub fn empresa_evaluar(
    id: &str,
    anterior: &str,
    nombre: &str,
    nombre_en_uso: bool,
) -> Result<String, JsValue> {
    regla_empresa(id, anterior, nombre, nombre_en_uso).map_err(a_js)
}

/// El nombre de empresa en su forma única, vacío si no vale (el nombre se compara así para
/// saber si ya existe).
#[wasm_bindgen]
pub fn empresa_nombre(texto: &str) -> String {
    NombreEmpresa::nuevo(texto)
        .map(|nombre| nombre.as_str().to_owned())
        .unwrap_or_default()
}

fn regla_praind_vencido(fecha: &str, ahora_ms: f64) -> bool {
    match (fecha.parse::<NaiveDate>(), hoy_de_costa_rica(ahora_ms)) {
        (Ok(fecha), Ok(hoy)) => praind_vencido(fecha, hoy),
        _ => false,
    }
}

/// ¿Está vencido el PRAIND que vence en `fecha` (`AAAA-MM-DD`)? Hoy es el de Costa Rica.
#[wasm_bindgen]
pub fn praind_esta_vencido(fecha: &str, ahora_ms: f64) -> bool {
    regla_praind_vencido(fecha, ahora_ms)
}

/// ¿Es un PRAIND de verdad, o la fecha de quien se registró sin acceso y sin PRAIND?
#[wasm_bindgen]
pub fn fecha_es_praind(fecha: &str) -> bool {
    fecha.parse::<NaiveDate>().is_ok_and(tiene_praind)
}

/// El nombre de empresa como se ve mientras se escribe (mayúscula, sin tocar los espacios).
#[wasm_bindgen]
pub fn empresa_al_escribir(texto: &str) -> String {
    nombre_empresa_mientras_se_escribe(texto)
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
    fn el_buscador_encuentra_por_nombre_y_por_cedula_sin_tildes_ni_mayusculas() {
        let filas = r#"[
            [{"campo":"IDENTIFICACION","texto":"111111111"},{"campo":"NOMBRE","texto":"JOSÉ PEÑA"}],
            [{"campo":"IDENTIFICACION","texto":"222222222"},{"campo":"NOMBRE","texto":"ANA MORA"}]
        ]"#;
        assert_eq!(
            regla_filas_que_coinciden("jose", filas),
            Ok("[true,false]".to_owned())
        );
        assert_eq!(
            regla_filas_que_coinciden("2222", filas),
            Ok("[false,true]".to_owned())
        );
        assert_eq!(
            regla_filas_que_coinciden("", filas),
            Ok("[true,true]".to_owned()),
            "un buscador vacío no excluye a nadie"
        );
        assert_eq!(
            regla_filas_que_coinciden("x", "no es json"),
            Err("json_invalido")
        );
        assert_eq!(
            regla_filas_que_coinciden("x", r#"[[{"campo":"OTRO","texto":"a"}]]"#),
            Err("campo_invalido")
        );
    }

    #[test]
    fn el_resaltado_parte_el_texto_y_lo_reconstruye_exacto() {
        let respuesta = json_de(&regla_resaltar("ana", "NOMBRE", r#"["ANA MORA"]"#).unwrap());
        let segmentos = respuesta[0].as_array().unwrap();
        let reconstruido: String = segmentos
            .iter()
            .map(|s| s["texto"].as_str().unwrap())
            .collect();
        assert_eq!(reconstruido, "ANA MORA");
        assert!(
            segmentos.iter().any(|s| s["resaltado"] == true),
            "algo se resalta: {segmentos:?}"
        );
        assert_eq!(regla_resaltar("ana", "RARO", "[]"), Err("campo_invalido"));
    }

    #[test]
    fn los_filtros_de_columna_ignoran_mayusculas_y_tildes() {
        let pedidos = r#"[
            {"operacion":"CONTIENE","celda":"Carlos Mauricio Sánchez","filtro":"carlos sa"},
            {"operacion":"IGUAL","celda":"Peña","filtro":"pena"},
            {"operacion":"NO_CONTIENE","celda":"Ana","filtro":"ana"},
            {"operacion":"EMPIEZA_CON","celda":"Ana","filtro":""}
        ]"#;
        assert_eq!(
            regla_celdas_que_cumplen(pedidos),
            Ok("[true,true,false,true]".to_owned())
        );
        assert_eq!(
            regla_celdas_que_cumplen(r#"[{"operacion":"RARA","celda":"a","filtro":"a"}]"#),
            Err("operacion_invalida")
        );
    }

    #[test]
    fn el_mensaje_de_un_error_de_usuario_es_el_del_nucleo() {
        assert_eq!(
            regla_mensaje_de_usuario("cedula_invalida"),
            "La cédula debe tener sólo números, entre 9 y 13 dígitos"
        );
        assert_eq!(regla_mensaje_de_usuario("nube"), "", "no es de los suyos");
    }

    #[test]
    fn el_estado_de_un_equipo_lo_decide_el_nucleo() {
        let visto = 1_760_450_400_000.0; // 2025-10-14T14:00:00Z
        let minutos = |n: f64| visto + n * 60_000.0;
        assert_eq!(
            regla_estado_de_conexion(true, visto, minutos(3.0)),
            Ok("en_linea".to_owned())
        );
        assert_eq!(
            regla_estado_de_conexion(true, visto, minutos(11.0)),
            Ok("caido".to_owned())
        );
        assert_eq!(
            regla_estado_de_conexion(false, visto, minutos(1.0)),
            Ok("fuera".to_owned())
        );
        assert_eq!(
            regla_estado_de_conexion(true, f64::MAX, minutos(1.0)),
            Err("fecha_invalida")
        );
    }

    #[test]
    fn el_buscador_se_ve_en_mayuscula() {
        assert_eq!(buscador_al_escribir("josé peña "), "JOSE PEÑA ");
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

    const EMPRESA: &str = "0199c7a2-0000-7000-8000-0000000000aa";
    const CONTRATISTA: &str = "0199c7a2-0000-7000-8000-0000000000bb";
    // 2026-10-14T12:00:00Z (en Costa Rica, las 06:00 del mismo día)
    const AHORA_MS: f64 = 1_791_979_200_000.0;

    fn pedido_de_contratista(extra: &serde_json::Value) -> String {
        let mut pedido = json!({
            "id": CONTRATISTA,
            "ahora_ms": AHORA_MS,
            "anterior": null,
            "borrador": {
                "cedula": "1-1111-1111",
                "nombre": " josé  peña ",
                "empresa": EMPRESA,
                "tipo_ingreso": "PRAIND",
                "fecha_vencimiento_praind": "2027-01-01",
                "tiene_acceso": true,
            },
            "hechos": { "cedula_en_uso": false, "empresa_existe": true, "esta_adentro": false },
        });
        if let (Some(destino), Some(origen)) = (pedido.as_object_mut(), extra.as_object()) {
            destino.extend(origen.clone());
        }
        pedido.to_string()
    }

    #[test]
    fn un_contratista_correcto_sale_normalizado_con_su_auditoria() {
        let respuesta = json_de(&regla_contratista(&pedido_de_contratista(&json!({}))).unwrap());
        assert_eq!(respuesta["problemas"], json!([]));
        assert_eq!(respuesta["datos"]["cedula"], "111111111");
        assert_eq!(respuesta["datos"]["nombre"], "JOSE PEÑA");
        assert_eq!(respuesta["datos"]["nombre_busqueda"], "JOSE PENA");
        assert_eq!(respuesta["datos"]["tipo_ingreso"], "PRAIND");
        assert_eq!(respuesta["cambios"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn el_formulario_vacio_marca_cada_campo_con_el_mensaje_del_nucleo() {
        let pedido = pedido_de_contratista(&json!({
            "borrador": { "cedula": "", "nombre": "", "empresa": "", "tipo_ingreso": "PRAIND",
                          "fecha_vencimiento_praind": "", "tiene_acceso": true },
        }));
        let respuesta = json_de(&regla_contratista(&pedido).unwrap());
        let campos: Vec<&str> = respuesta["problemas"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["campo"].as_str().unwrap())
            .collect();
        assert_eq!(
            campos,
            ["cedula", "nombre", "empresa_id", "fecha_vencimiento_praind"]
        );
        assert_eq!(
            respuesta["problemas"][0]["mensaje"],
            "La cédula es obligatoria"
        );
    }

    #[test]
    fn el_praind_vencido_se_mide_con_el_dia_de_costa_rica() {
        let vence_ayer = json!({ "borrador": {
            "cedula": "1-1111-1111", "nombre": "ana", "empresa": EMPRESA,
            "tipo_ingreso": "PRAIND", "fecha_vencimiento_praind": "2026-10-13",
            "tiene_acceso": true } });
        let respuesta = json_de(&regla_contratista(&pedido_de_contratista(&vence_ayer)).unwrap());
        assert_eq!(respuesta["problemas"][0]["codigo"], "praind_vencido");

        let vence_hoy = json!({ "borrador": {
            "cedula": "1-1111-1111", "nombre": "ana", "empresa": EMPRESA,
            "tipo_ingreso": "PRAIND", "fecha_vencimiento_praind": "2026-10-14",
            "tiene_acceso": true } });
        let respuesta = json_de(&regla_contratista(&pedido_de_contratista(&vence_hoy)).unwrap());
        assert_eq!(respuesta["problemas"], json!([]));
    }

    #[test]
    fn editar_dice_que_cambio_y_respeta_las_reglas_de_la_edicion() {
        let anterior = json!({
            "id": CONTRATISTA, "cedula": "111111111", "nombre": "JOSE PEÑA", "empresa": EMPRESA,
            "tipo_ingreso": "PRAIND", "fecha_vencimiento_praind": "2027-01-01",
            "tiene_acceso": true,
        });
        let cambiar_nombre = json!({ "anterior": anterior, "borrador": {
            "cedula": "111111111", "nombre": "jose pena", "empresa": EMPRESA,
            "tipo_ingreso": "PRAIND", "fecha_vencimiento_praind": "2027-01-01",
            "tiene_acceso": false } });
        let respuesta =
            json_de(&regla_contratista(&pedido_de_contratista(&cambiar_nombre)).unwrap());
        let campos: Vec<&str> = respuesta["cambios"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["campo"].as_str().unwrap())
            .collect();
        assert_eq!(campos, ["nombre", "tiene_acceso"]);

        let otra_cedula_adentro = json!({
            "anterior": anterior,
            "borrador": { "cedula": "222222222", "nombre": "JOSE PEÑA", "empresa": EMPRESA,
                          "tipo_ingreso": "PRAIND", "fecha_vencimiento_praind": "2027-01-01",
                          "tiene_acceso": true },
            "hechos": { "cedula_en_uso": false, "empresa_existe": true, "esta_adentro": true },
        });
        let respuesta =
            json_de(&regla_contratista(&pedido_de_contratista(&otra_cedula_adentro)).unwrap());
        assert_eq!(
            respuesta["problemas"][0]["codigo"],
            "cedula_no_editable_adentro"
        );
    }

    #[test]
    fn lo_mal_formado_no_llega_a_las_reglas() {
        assert_eq!(regla_contratista("no es json"), Err("json_invalido"));
        let sin_id = pedido_de_contratista(&json!({ "id": "x" }));
        assert_eq!(regla_contratista(&sin_id), Err("id_invalido"));
        assert_eq!(regla_cedula_de_contratista("1-1111-1111"), "111111111");
        assert_eq!(regla_cedula_de_contratista("AB1"), "");
    }

    #[test]
    fn quien_se_registra_sin_acceso_no_lleva_praind() {
        let sin_acceso = json!({ "borrador": {
            "cedula": "1-1111-1111", "nombre": "ana", "empresa": EMPRESA,
            "tipo_ingreso": "PRAIND", "fecha_vencimiento_praind": "",
            "tiene_acceso": false } });
        let respuesta = json_de(&regla_contratista(&pedido_de_contratista(&sin_acceso)).unwrap());
        assert_eq!(respuesta["problemas"], json!([]));
        assert_eq!(respuesta["datos"]["fecha_vencimiento_praind"], "1970-01-01");
        assert!(!fecha_es_praind("1970-01-01"));
        assert!(fecha_es_praind("2027-01-01"));
    }

    #[test]
    fn el_praind_vencido_lo_dice_el_nucleo() {
        assert!(regla_praind_vencido("2026-10-13", AHORA_MS));
        assert!(
            !regla_praind_vencido("2026-10-14", AHORA_MS),
            "vence hoy: vigente"
        );
        assert!(!regla_praind_vencido("no es fecha", AHORA_MS));
    }

    #[test]
    fn el_nombre_de_una_empresa_sale_en_mayuscula_y_no_se_repite() {
        let alta = json_de(&regla_empresa(EMPRESA, "", "  acme   s.a. ", false).unwrap());
        assert_eq!(alta["nombre"], "ACME S.A.");
        assert_eq!(alta["cambios"].as_array().unwrap().len(), 1);

        let repetida = json_de(&regla_empresa(EMPRESA, "", "acme", true).unwrap());
        assert_eq!(repetida["problema"]["codigo"], "empresa_nombre_repetido");
        assert_eq!(repetida["problema"]["campo"], "nombre");
        assert_eq!(
            repetida["problema"]["mensaje"],
            "Ya existe una empresa con ese nombre"
        );

        let vacia = json_de(&regla_empresa(EMPRESA, "", " ", false).unwrap());
        assert_eq!(vacia["problema"]["codigo"], "empresa_nombre_vacio");

        let renombre = json_de(&regla_empresa(EMPRESA, "ACME", "acme norte", false).unwrap());
        assert_eq!(renombre["cambios"][0]["despues"], "ACME NORTE");
        assert_eq!(empresa_nombre(" acme  norte"), "ACME NORTE");
        assert_eq!(empresa_al_escribir("acme "), "ACME ");
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
