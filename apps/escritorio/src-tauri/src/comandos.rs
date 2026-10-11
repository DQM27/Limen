//! Los comandos que ve la interfaz (`invoke`). Cada uno es una función
//! delgada: pide la sesión, llama al método del mismo nombre de
//! [`limen_escritorio_comandos::Comandos`] y devuelve su resultado. La lógica
//! nunca vive aquí; si un comando necesita algo más que eso, va a `Comandos`
//! para poder probarlo sin abrir una ventana.
//!
//! Tauri convierte los nombres de los parámetros a `camelCase` en el lado de
//! JavaScript (`ingreso_id` se invoca como `ingresoId`); los campos de los
//! objetos JSON se quedan en `snake_case`.

// `#[tauri::command]` genera un `unreachable!` dentro de cada comando `async`
// que devuelve `Result`; no es código nuestro.
#![expect(
    clippy::unreachable,
    reason = "lo genera la macro #[tauri::command] en los comandos async"
)]

use limen_escritorio_comandos::{
    AtajoFechaDto, CambioDto, CambioGafeteEntrada, CandidatoIngresoDto, ContratistaDto,
    ContratistaEntrada, EmpresaDto, EmpresaProveedoraDto, EntradaContratistaEntrada,
    EntradaCorreoEntrada, EntradaProveedorEntrada, EntradaRegistradaDto, ErrorJson,
    FilaContratistaDto, GafeteDto, HistorialDto, PersonaAdentroDto, PersonalKofDto, UsuarioActual,
};
use tauri::window::Color;
use tauri::{LogicalSize, State, WebviewWindow};

use crate::estado::Estado;

type Resultado<T> = Result<T, ErrorJson>;

// --- Ventana ---

/// Tamaño de la ventana de entrada (splash e inicio de sesión), sin marco.
const ENTRADA: LogicalSize<f64> = LogicalSize::new(380.0, 480.0);

/// Pone la ventana en su forma de "entrada" (pequeña, sin marco, centrada: el
/// splash y el inicio de sesión) y la muestra, o de "aplicacion" (con marco y
/// maximizada).
/// No decide nada de negocio: sólo da forma a la ventana.
#[tauri::command]
pub fn modo_ventana(ventana: WebviewWindow, modo: String) -> Result<(), String> {
    let error = |e: tauri::Error| e.to_string();
    match modo.as_str() {
        "entrada" => {
            ventana.unmaximize().map_err(error)?;
            ventana.set_resizable(false).map_err(error)?;
            ventana.set_decorations(false).map_err(error)?;
            ventana.set_size(ENTRADA).map_err(error)?;
            // El fondo de la ventana, del color de la tarjeta según el tema
            // del sistema: si algún borde queda sin cubrir, no se ve blanco.
            let fondo = match ventana.theme().map_err(error)? {
                tauri::Theme::Dark => Color(0x1f, 0x23, 0x35, 0xff),
                _ => Color(0xe1, 0xe2, 0xe7, 0xff),
            };
            ventana.set_background_color(Some(fondo)).map_err(error)?;
            ventana.center().map_err(error)?;
            // La ventana nace oculta: se muestra cuando la interfaz ya pintó
            // el splash, para que nunca se vea un rectángulo en blanco.
            ventana.show().map_err(error)
        }
        "aplicacion" => {
            ventana.set_decorations(true).map_err(error)?;
            ventana.set_resizable(true).map_err(error)?;
            ventana.maximize().map_err(error)
        }
        otro => Err(format!("modo de ventana desconocido: {otro}")),
    }
}

/// Cierra la app (el botón "Cancelar" de la ventana de entrada, que no tiene
/// marco con su propio botón de cerrar).
#[tauri::command]
pub fn salir(app: tauri::AppHandle) {
    app.exit(0);
}

// --- Sesión (bloque L) ---

#[tauri::command]
pub async fn iniciar_sesion(
    estado: State<'_, Estado>,
    cedula: String,
    clave: String,
) -> Resultado<UsuarioActual> {
    estado.comandos().iniciar_sesion(&cedula, &clave).await
}

#[tauri::command]
pub fn cerrar_sesion(estado: State<'_, Estado>) {
    estado.comandos().cerrar_sesion();
}

/// Quién tiene la sesión en este equipo, si alguien.
#[tauri::command]
pub fn usuario_actual(estado: State<'_, Estado>) -> Option<UsuarioActual> {
    estado.comandos().usuario_actual()
}

/// Cambia la clave propia; es lo único permitido con una temporal.
#[tauri::command]
pub async fn cambiar_clave(
    estado: State<'_, Estado>,
    clave_actual: String,
    clave: String,
) -> Resultado<()> {
    estado.comandos().cambiar_clave(&clave_actual, &clave).await
}

// --- Lecturas ---

#[tauri::command]
pub async fn dentro(estado: State<'_, Estado>) -> Resultado<Vec<PersonaAdentroDto>> {
    estado.comandos().sesion()?;
    estado.comandos().dentro().await
}

/// Los ingresos y salidas entre `desde` y `hasta` (`AAAA-MM-DD`, vacíos =
/// sin límite).
#[tauri::command]
pub async fn listar_historial(
    estado: State<'_, Estado>,
    desde: Option<String>,
    hasta: Option<String>,
) -> Resultado<HistorialDto> {
    estado.comandos().sesion()?;
    estado
        .comandos()
        .listar_historial(desde.as_deref(), hasta.as_deref())
        .await
}

/// Los accesos rápidos de fecha del historial.
#[tauri::command]
pub fn atajos_de_fecha(estado: State<'_, Estado>) -> Resultado<Vec<AtajoFechaDto>> {
    estado.comandos().sesion()?;
    Ok(estado.comandos().atajos_de_fecha())
}

#[tauri::command]
pub async fn listar_contratistas(estado: State<'_, Estado>) -> Resultado<Vec<FilaContratistaDto>> {
    estado.comandos().sesion()?;
    estado.comandos().listar_contratistas().await
}

#[tauri::command]
pub async fn buscar_contratistas(
    estado: State<'_, Estado>,
    texto: String,
    limite: usize,
) -> Resultado<Vec<ContratistaDto>> {
    estado.comandos().sesion()?;
    estado.comandos().buscar_contratistas(&texto, limite).await
}

#[tauri::command]
pub async fn buscar_empresas(
    estado: State<'_, Estado>,
    texto: String,
    limite: usize,
) -> Resultado<Vec<EmpresaDto>> {
    estado.comandos().sesion()?;
    estado.comandos().buscar_empresas(&texto, limite).await
}

// --- Escrituras ---

#[tauri::command]
pub async fn registrar_salida(
    estado: State<'_, Estado>,
    via: String,
    ingreso_id: String,
) -> Resultado<()> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_salida(&sesion, &via, &ingreso_id)
        .await
}

#[tauri::command]
pub async fn registrar_salida_por_gafete(
    estado: State<'_, Estado>,
    via: String,
    numero: u32,
) -> Resultado<()> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_salida_por_gafete(&sesion, &via, numero)
        .await
}

#[tauri::command]
pub async fn registrar_empresa(estado: State<'_, Estado>, nombre: String) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado.comandos().registrar_empresa(&sesion, &nombre).await
}

#[tauri::command]
pub async fn registrar_contratista(
    estado: State<'_, Estado>,
    contratista: ContratistaEntrada,
) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_contratista(&sesion, &contratista)
        .await
}

#[tauri::command]
pub async fn editar_contratista(
    estado: State<'_, Estado>,
    id: String,
    contratista: ContratistaEntrada,
) -> Resultado<Vec<CambioDto>> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .editar_contratista(&sesion, &id, &contratista)
        .await
}

#[tauri::command]
pub async fn registrar_entrada_contratista(
    estado: State<'_, Estado>,
    entrada: EntradaContratistaEntrada,
) -> Resultado<EntradaRegistradaDto> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_entrada_contratista(&sesion, &entrada)
        .await
}

// --- Empresas, proveedores, correo, personal KOF y gafetes ---

#[tauri::command]
pub async fn renombrar_empresa(
    estado: State<'_, Estado>,
    id: String,
    nombre: String,
) -> Resultado<Vec<CambioDto>> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .renombrar_empresa(&sesion, &id, &nombre)
        .await
}

#[tauri::command]
pub async fn buscar_empresas_proveedoras(
    estado: State<'_, Estado>,
    texto: String,
    limite: usize,
) -> Resultado<Vec<EmpresaProveedoraDto>> {
    estado.comandos().sesion()?;
    estado
        .comandos()
        .buscar_empresas_proveedoras(&texto, limite)
        .await
}

#[tauri::command]
pub async fn registrar_empresa_proveedora(
    estado: State<'_, Estado>,
    nombre: String,
) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_empresa_proveedora(&sesion, &nombre)
        .await
}

#[tauri::command]
pub async fn renombrar_empresa_proveedora(
    estado: State<'_, Estado>,
    id: String,
    nombre: String,
) -> Resultado<Vec<CambioDto>> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .renombrar_empresa_proveedora(&sesion, &id, &nombre)
        .await
}

#[tauri::command]
pub async fn registrar_entrada_proveedor(
    estado: State<'_, Estado>,
    entrada: EntradaProveedorEntrada,
) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_entrada_proveedor(&sesion, &entrada)
        .await
}

#[tauri::command]
pub async fn registrar_entrada_correo(
    estado: State<'_, Estado>,
    entrada: EntradaCorreoEntrada,
) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_entrada_correo(&sesion, &entrada)
        .await
}

#[tauri::command]
pub async fn buscar_personal_kof(
    estado: State<'_, Estado>,
    texto: String,
    limite: usize,
) -> Resultado<Vec<PersonalKofDto>> {
    estado.comandos().sesion()?;
    estado.comandos().buscar_personal_kof(&texto, limite).await
}

#[tauri::command]
pub async fn registrar_personal_kof(
    estado: State<'_, Estado>,
    codigo_empleado: String,
    nombre: String,
) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_personal_kof(&sesion, &codigo_empleado, &nombre)
        .await
}

#[tauri::command]
pub async fn editar_personal_kof(
    estado: State<'_, Estado>,
    id: String,
    nombre: String,
    activo: bool,
) -> Resultado<Vec<CambioDto>> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .editar_personal_kof(&sesion, &id, &nombre, activo)
        .await
}

#[tauri::command]
pub async fn entregar_gafete_kof(
    estado: State<'_, Estado>,
    personal_id: String,
    gafete: u32,
) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .entregar_gafete_kof(&sesion, &personal_id, gafete)
        .await
}

#[tauri::command]
pub async fn listar_gafetes(estado: State<'_, Estado>, tipo: String) -> Resultado<Vec<GafeteDto>> {
    estado.comandos().sesion()?;
    estado.comandos().listar_gafetes(&tipo).await
}

#[tauri::command]
pub async fn registrar_gafetes(
    estado: State<'_, Estado>,
    tipo: String,
    desde: u32,
    hasta: u32,
) -> Resultado<usize> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .registrar_gafetes(&sesion, &tipo, desde, hasta)
        .await
}

#[tauri::command]
pub async fn cambiar_gafete(
    estado: State<'_, Estado>,
    tipo: String,
    numero: u32,
    cambio: CambioGafeteEntrada,
) -> Resultado<Vec<CambioDto>> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .cambiar_gafete(&sesion, &tipo, numero, &cambio)
        .await
}

// --- Preparar el ingreso de un contratista ---

#[tauri::command]
pub async fn buscar_para_ingreso(
    estado: State<'_, Estado>,
    texto: String,
    limite: usize,
) -> Resultado<Vec<CandidatoIngresoDto>> {
    estado.comandos().sesion()?;
    estado.comandos().buscar_para_ingreso(&texto, limite).await
}

#[tauri::command]
pub async fn preparar_ingreso(
    estado: State<'_, Estado>,
    contratista_id: String,
) -> Resultado<CandidatoIngresoDto> {
    estado.comandos().sesion()?;
    estado.comandos().preparar_ingreso(&contratista_id).await
}
