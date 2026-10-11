//! Cascarón de Tauri de la app móvil de Limen.
//!
//! Prueba de concepto: reutiliza tal cual los comandos y el estado del
//! escritorio (`#[path]`) y la misma interfaz de Angular. La primera vez que
//! arranca, si trae `semilla.zip` (una copia de la base de pruebas del
//! escritorio, generada por `preparar.ps1` y nunca subida al repositorio), la
//! abre como su base.

#[path = "../../../escritorio/src-tauri/src/comandos.rs"]
mod comandos;
mod estado;

use std::io::Cursor;
use std::path::Path;

use limen_composicion::{AplicacionLimen, Config};
use limen_infra_plataforma::{RelojConfiable, SERVIDORES_NTP, leer_desfase};
use tauri::{AppHandle, Manager};

use estado::Estado;

const CARPETA_BASE: &str = "base";
const ARCHIVO_DESFASE_RELOJ: &str = "reloj-desfase.txt";
const SEMILLA: &str = "semilla.zip";

/// Si la base todavía no existe y la app trae semilla, la descomprime.
fn sembrar_base(app: &AppHandle, ruta_base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if ruta_base.join("manifest").exists() {
        return Ok(());
    }
    let Some(semilla) = app.asset_resolver().get(SEMILLA.to_owned()) else {
        return Ok(());
    };
    std::fs::create_dir_all(ruta_base)?;
    zip::ZipArchive::new(Cursor::new(semilla.bytes))?.extract(ruta_base)?;
    Ok(())
}

/// Siembra y abre la base; avisa al estado cuando está lista.
async fn abrir_base(app: AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let datos = app.path().app_data_dir()?;
    std::fs::create_dir_all(&datos)?;
    let ruta_base = datos.join(CARPETA_BASE);
    let ruta = ruta_base.clone();
    let manejador = app.clone();
    tauri::async_runtime::spawn_blocking(move || sembrar_base(&manejador, &ruta).map_err(|e| e.to_string()))
        .await?
        .map_err(Box::<dyn std::error::Error + Send + Sync>::from)?;
    let ruta_desfase = datos.join(ARCHIVO_DESFASE_RELOJ);
    let reloj = RelojConfiable::new(leer_desfase(&ruta_desfase));
    reloj.sincronizar_en_segundo_plano(&SERVIDORES_NTP, ruta_desfase);
    let aplicacion = AplicacionLimen::abrir(&Config { ruta_base }, &reloj).await?;
    app.state::<Estado>().listo(aplicacion);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            app.manage(Estado::default());
            let manejador = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = abrir_base(manejador.clone()).await {
                    tauri_plugin_log::log::error!("no se pudo abrir la base: {error}");
                    manejador.exit(1);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::iniciar_sesion,
            comandos::cerrar_sesion,
            comandos::usuario_actual,
            comandos::cambiar_clave,
            comandos::dentro,
            comandos::listar_contratistas,
            comandos::listar_historial,
            comandos::atajos_de_fecha,
            comandos::buscar_contratistas,
            comandos::buscar_empresas,
            comandos::registrar_salida,
            comandos::registrar_salida_por_gafete,
            comandos::registrar_empresa,
            comandos::registrar_contratista,
            comandos::editar_contratista,
            comandos::registrar_entrada_contratista,
            comandos::buscar_para_ingreso,
            comandos::preparar_ingreso,
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
        .expect("error al ejecutar la aplicación móvil");
}
