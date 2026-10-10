use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

/// La hora. El dominio nunca la consulta: el caso de uso se la pasa.
pub trait Reloj: Send + Sync {
    /// Instante actual, en UTC.
    fn ahora(&self) -> DateTime<Utc>;
    /// Fecha de hoy en la zona de la instalación (Costa Rica), que es la que
    /// usan las reglas de calendario (vencimiento del PRAIND).
    fn hoy(&self) -> NaiveDate;
    /// El primer instante de `fecha` en la zona de la instalación, en UTC:
    /// con él se convierten los días que elige la persona (de Costa Rica) en
    /// los instantes con que se guardan los movimientos.
    fn inicio_del_dia(&self, fecha: NaiveDate) -> DateTime<Utc>;
}

/// IDs nuevos para entidades. En producción, UUID v7.
pub trait GeneradorIds: Send + Sync {
    fn nuevo(&self) -> Uuid;
}
