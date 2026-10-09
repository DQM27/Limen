//! Veto por persona (regla A8): lo averigua el caso de uso y lo decide el
//! dominio ([`limen_dominio::visitante::verificar_veto`]).

use limen_dominio::cedula::Cedula;

use crate::puertos::{ErrorPersistencia, RepositorioContratistas};

/// Si la cédula tiene el acceso denegado. Hoy el único lugar donde se niega
/// el acceso a una persona es su ficha de contratista.
pub async fn cedula_vetada<C: RepositorioContratistas>(
    contratistas: &C,
    cedula: &Cedula,
) -> Result<bool, ErrorPersistencia> {
    Ok(contratistas
        .obtener_por_cedula(cedula)
        .await?
        .is_some_and(|contratista| !contratista.tiene_acceso()))
}
