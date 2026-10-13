//! Cascarón de Tauri de la app de escritorio de Limen.
//!
//! Sólo hace tres cosas: arranca la aplicación de este equipo con
//! `AplicacionLimen::arrancar`, la guarda como estado compartido y registra
//! los comandos. Además, cuando bajan cambios de la nube, se lo avisa a la
//! interfaz (`nube-cambios`) para que recargue lo que muestra. Las reglas viven en `crates/dominio` y la lógica de cada comando
//! en `limen-escritorio-comandos`.

mod comandos;
mod estado;

use limen_composicion::{AplicacionLimen, Config};
use tauri::{Emitter, Manager};

use estado::Estado;

/// Carpeta de la base dentro de los datos de la app.
const CARPETA_BASE: &str = "base";
/// El último desfase medido del reloj del equipo, para la próxima apertura.
const ARCHIVO_DESFASE_RELOJ: &str = "reloj-desfase.txt";
/// La identidad del equipo: su unidad operativa y su ID.
const ARCHIVO_EQUIPO: &str = "equipo.txt";
/// El evento con que la interfaz se entera de que bajaron cambios de la nube
/// (de otros equipos) y recarga lo que muestra.
const EVENTO_CAMBIOS_DE_LA_NUBE: &str = "nube-cambios";
/// Cuánto puede seguir oculta la ventana si la interfaz no la muestra.
const ESPERA_MAXIMA_VENTANA_OCULTA: std::time::Duration = std::time::Duration::from_secs(5);

/// Abre la base. Si falla no hay app que mostrar: el error sube a `setup`,
/// que detiene el arranque con el motivo escrito. La app arranca sin
/// sesión: la interfaz pide entrar. Al desarrollar, la base se siembra con
/// usuarios de prueba (`limen_composicion::semilla`).
fn preparar_estado(app: &tauri::App) -> Result<Estado, Box<dyn std::error::Error>> {
    let datos = app.path().app_data_dir()?;
    std::fs::create_dir_all(&datos)?;
    let ventanas = app.handle().clone();
    let avisar_a_la_interfaz = move || {
        // Si la ventana ya se cerró, no hay a quién avisar.
        drop(ventanas.emit(EVENTO_CAMBIOS_DE_LA_NUBE, ()));
    };
    let aplicacion = tauri::async_runtime::block_on(AplicacionLimen::arrancar(
        &Config {
            ruta_base: datos.join(CARPETA_BASE),
            ruta_desfase_reloj: datos.join(ARCHIVO_DESFASE_RELOJ),
            ruta_equipo: datos.join(ARCHIVO_EQUIPO),
        },
        avisar_a_la_interfaz,
    ))?;
    Ok(Estado::new(aplicacion))
}

/// La ventana nace oculta y la interfaz la muestra (`modo_ventana`) cuando ya
/// pintó el splash. Si por una falla la interfaz nunca avisa, esta red de
/// seguridad la muestra pasado un rato, para que la app no quede invisible.
fn mostrar_si_la_interfaz_no_avisa(app: &tauri::App) {
    let Some(ventana) = app.get_webview_window("main") else {
        return;
    };
    std::thread::spawn(move || {
        std::thread::sleep(ESPERA_MAXIMA_VENTANA_OCULTA);
        drop(ventana.show());
    });
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
            mostrar_si_la_interfaz_no_avisa(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::modo_ventana,
            comandos::salir,
            comandos::estado_vinculacion_equipo,
            comandos::vincular_equipo_con_codigo,
            comandos::reiniciar,
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
            comandos::resaltar_coincidencias,
            comandos::filas_que_coinciden,
            comandos::celdas_que_cumplen,
            comandos::registrar_salida,
            comandos::registrar_salida_por_gafete,
            comandos::registrar_empresa,
            comandos::registrar_contratista,
            comandos::editar_contratista,
            comandos::validar_contratista,
            comandos::validar_entrada_proveedor,
            comandos::validar_entrada_correo,
            comandos::validar_personal_kof,
            comandos::validar_entrega_gafete_kof,
            comandos::nombre_persona_al_escribir,
            comandos::nombre_empresa_al_escribir,
            comandos::placa_al_escribir,
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
        .expect("error al ejecutar la aplicación de escritorio");
}
