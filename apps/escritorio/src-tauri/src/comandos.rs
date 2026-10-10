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
    CambioDto, CambioGafeteEntrada, ContratistaDto, ContratistaEntrada, EmpresaDto,
    EmpresaProveedoraDto, EntradaContratistaEntrada, EntradaCorreoEntrada, EntradaProveedorEntrada,
    EntradaRegistradaDto, ErrorJson, FilaContratistaDto, GafeteDto, Operador, PersonaAdentroDto,
    PersonalKofDto,
};
use tauri::State;

use crate::estado::Estado;

type Resultado<T> = Result<T, ErrorJson>;

// --- Operador provisional (TEMPORAL, hasta el bloque L) ---

/// El operador de este equipo (el arranque lo crea si no existía).
#[tauri::command]
pub fn operador_actual(estado: State<'_, Estado>) -> Option<Operador> {
    estado.operador().actual()
}

// --- Lecturas ---

#[tauri::command]
pub async fn dentro(estado: State<'_, Estado>) -> Resultado<Vec<PersonaAdentroDto>> {
    estado.operador().sesion()?;
    estado.comandos().dentro().await
}

#[tauri::command]
pub async fn listar_contratistas(estado: State<'_, Estado>) -> Resultado<Vec<FilaContratistaDto>> {
    estado.operador().sesion()?;
    estado.comandos().listar_contratistas().await
}

#[tauri::command]
pub async fn buscar_contratistas(
    estado: State<'_, Estado>,
    texto: String,
    limite: usize,
) -> Resultado<Vec<ContratistaDto>> {
    estado.operador().sesion()?;
    estado.comandos().buscar_contratistas(&texto, limite).await
}

#[tauri::command]
pub async fn buscar_empresas(
    estado: State<'_, Estado>,
    texto: String,
    limite: usize,
) -> Resultado<Vec<EmpresaDto>> {
    estado.operador().sesion()?;
    estado.comandos().buscar_empresas(&texto, limite).await
}

// --- Escrituras ---

#[tauri::command]
pub async fn registrar_salida(
    estado: State<'_, Estado>,
    via: String,
    ingreso_id: String,
) -> Resultado<()> {
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
    estado
        .comandos()
        .registrar_salida_por_gafete(&sesion, &via, numero)
        .await
}

#[tauri::command]
pub async fn registrar_empresa(estado: State<'_, Estado>, nombre: String) -> Resultado<String> {
    let sesion = estado.operador().sesion()?;
    estado.comandos().registrar_empresa(&sesion, &nombre).await
}

#[tauri::command]
pub async fn registrar_contratista(
    estado: State<'_, Estado>,
    contratista: ContratistaEntrada,
) -> Resultado<String> {
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    estado.operador().sesion()?;
    estado.comandos().buscar_personal_kof(&texto, limite).await
}

#[tauri::command]
pub async fn registrar_personal_kof(
    estado: State<'_, Estado>,
    codigo_empleado: String,
    nombre: String,
) -> Resultado<String> {
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
    estado
        .comandos()
        .entregar_gafete_kof(&sesion, &personal_id, gafete)
        .await
}

#[tauri::command]
pub async fn listar_gafetes(estado: State<'_, Estado>, tipo: String) -> Resultado<Vec<GafeteDto>> {
    estado.operador().sesion()?;
    estado.comandos().listar_gafetes(&tipo).await
}

#[tauri::command]
pub async fn registrar_gafetes(
    estado: State<'_, Estado>,
    tipo: String,
    desde: u32,
    hasta: u32,
) -> Resultado<usize> {
    let sesion = estado.operador().sesion()?;
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
    let sesion = estado.operador().sesion()?;
    estado
        .comandos()
        .cambiar_gafete(&sesion, &tipo, numero, &cambio)
        .await
}
