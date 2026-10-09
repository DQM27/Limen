use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

/// La hora. El dominio nunca la consulta: el caso de uso se la pasa.
pub trait Reloj: Send + Sync {
    /// Instante actual, en UTC.
    fn ahora(&self) -> DateTime<Utc>;
    /// Fecha de hoy en la zona de la instalación (Costa Rica), que es la que
    /// usan las reglas de calendario (vencimiento del PRAIND).
    fn hoy(&self) -> NaiveDate;
}

/// IDs nuevos para entidades. En producción, UUID v7.
pub trait GeneradorIds: Send + Sync {
    fn nuevo(&self) -> Uuid;
}
