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
    ContratistaDto, ContratistaEntrada, EmpresaDto, EntradaContratistaEntrada,
    EntradaRegistradaDto, ErrorJson, Operador, PersonaAdentroDto,
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
