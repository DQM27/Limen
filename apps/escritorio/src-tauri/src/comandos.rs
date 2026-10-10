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
    UsuarioDto, UsuarioEntrada,
};
use tauri::State;

use crate::estado::Estado;

type Resultado<T> = Result<T, ErrorJson>;

// --- Sesión (bloque L) ---

/// Si el equipo ya tiene usuarios; sin ninguno, la interfaz ofrece crear
/// el primero. No pide sesión.
#[tauri::command]
pub async fn hay_usuarios(estado: State<'_, Estado>) -> Resultado<bool> {
    estado.comandos().hay_usuarios().await
}

#[tauri::command]
pub async fn crear_primer_usuario(
    estado: State<'_, Estado>,
    usuario: UsuarioEntrada,
) -> Resultado<UsuarioActual> {
    estado.comandos().crear_primer_usuario(&usuario).await
}

#[tauri::command]
pub async fn iniciar_sesion(
    estado: State<'_, Estado>,
    cedula: String,
    contrasena: String,
) -> Resultado<UsuarioActual> {
    estado.comandos().iniciar_sesion(&cedula, &contrasena).await
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

/// Cambia la contraseña propia; es lo único permitido con una temporal.
#[tauri::command]
pub async fn cambiar_contrasena(
    estado: State<'_, Estado>,
    contrasena_actual: String,
    contrasena: String,
) -> Resultado<()> {
    estado
        .comandos()
        .cambiar_contrasena(&contrasena_actual, &contrasena)
        .await
}

// --- Usuarios (TEMPORAL: se administran en el equipo hasta el panel de la nube) ---

#[tauri::command]
pub async fn listar_usuarios(estado: State<'_, Estado>) -> Resultado<Vec<UsuarioDto>> {
    estado.comandos().sesion()?;
    estado.comandos().listar_usuarios().await
}

#[tauri::command]
pub async fn registrar_usuario(
    estado: State<'_, Estado>,
    usuario: UsuarioEntrada,
) -> Resultado<String> {
    let sesion = estado.comandos().sesion()?;
    estado.comandos().registrar_usuario(&sesion, &usuario).await
}

#[tauri::command]
pub async fn editar_usuario(
    estado: State<'_, Estado>,
    id: String,
    nombre: String,
    activo: bool,
) -> Resultado<Vec<CambioDto>> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .editar_usuario(&sesion, &id, &nombre, activo)
        .await
}

#[tauri::command]
pub async fn restablecer_contrasena(
    estado: State<'_, Estado>,
    id: String,
    contrasena: String,
) -> Resultado<()> {
    let sesion = estado.comandos().sesion()?;
    estado
        .comandos()
        .restablecer_contrasena(&sesion, &id, &contrasena)
        .await
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
