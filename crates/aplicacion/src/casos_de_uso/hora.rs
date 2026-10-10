//! La hora con que se sella cada movimiento (regla E5): el caso de uso
//! averigua el último movimiento y el dominio decide.

use limen_dominio::reloj::{HoraSellada, sellar_hora};

use crate::puertos::{ErrorPersistencia, Reloj, RepositorioReloj, UnidadDeTrabajo};

pub(super) async fn sellar<U: UnidadDeTrabajo>(
    uow: &mut U,
    reloj: &impl Reloj,
) -> Result<HoraSellada, ErrorPersistencia> {
    let ultimo_movimiento = uow.reloj().ultimo_movimiento().await?;
    Ok(sellar_hora(reloj.lectura(), ultimo_movimiento))
}
