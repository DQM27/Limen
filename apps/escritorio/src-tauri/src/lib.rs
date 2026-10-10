//! Cascarón de Tauri de la app de escritorio de Limen.
//!
//! Sólo hace tres cosas: abre la base de este equipo con
//! `AplicacionLimen::abrir`, la guarda como estado compartido y registra los
//! comandos. Las reglas viven en `crates/dominio` y la lógica de cada comando
//! en `limen-escritorio-comandos`.

mod comandos;
mod estado;

use limen_composicion::{AplicacionLimen, Config};
use limen_escritorio_comandos::OperadorDelEquipo;
use limen_infra_plataforma::IdsV7;
use tauri::Manager;

use estado::Estado;

/// Carpeta de la base dentro de los datos de la app.
const CARPETA_BASE: &str = "base";
/// Archivo del operador provisional (TEMPORAL, hasta el bloque L).
const ARCHIVO_OPERADOR: &str = "operador.json";

/// Abre la base y el operador guardado. Si algo falla no hay app que mostrar:
/// el error sube a `setup`, que detiene el arranque con el motivo escrito.
fn preparar_estado(app: &tauri::App) -> Result<Estado, Box<dyn std::error::Error>> {
    let datos = app.path().app_data_dir()?;
    std::fs::create_dir_all(&datos)?;
    let operador = OperadorDelEquipo::abrir(datos.join(ARCHIVO_OPERADOR))?;
    // TEMPORAL: sin inicio de sesión, el primer arranque crea el operador.
    operador.asegurar_provisional(&IdsV7)?;
    let aplicacion = tauri::async_runtime::block_on(AplicacionLimen::abrir(&Config {
        ruta_base: datos.join(CARPETA_BASE),
    }))?;
    Ok(Estado::new(aplicacion, operador))
}

/// Arranca la aplicación y no vuelve hasta que se cierra la ventana.
///
/// # Panics
///
/// Si Tauri no puede arrancar (por ejemplo, la base de datos no abre).
#[expect(
    clippy::expect_used,
    reason = "si Tauri no arranca no hay nada que mostrar; el motivo queda escrito en el mensaje"
)]
pub fn run() {
    tauri::Builder::default()
        // `info`: sin esto el motor de la base llena el registro de líneas TRACE.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let estado = preparar_estado(app)?;
            app.manage(estado);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::operador_actual,
            comandos::dentro,
            comandos::listar_contratistas,
            comandos::buscar_contratistas,
            comandos::buscar_empresas,
            comandos::registrar_salida,
            comandos::registrar_salida_por_gafete,
            comandos::registrar_empresa,
            comandos::registrar_contratista,
            comandos::editar_contratista,
            comandos::registrar_entrada_contratista,
            comandos::renombrar_empresa,
            comandos::buscar_empresas_proveedoras,
            comandos::registrar_empresa_proveedora,
            comandos::renombrar_empresa_proveedora,
            comandos::registrar_entrada_proveedor,
            comandos::registrar_entrada_correo,
            comandos::buscar_personal_kof,
            comandos::registrar_personal_kof,
            comandos::editar_personal_kof,
            comandos::entregar_gafete_kof,
            comandos::listar_gafetes,
            comandos::registrar_gafetes,
            comandos::cambiar_gafete,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación de escritorio");
}
