//! Adaptadores de plataforma: lo único del núcleo que lee la hora del
//! sistema y genera IDs al azar. El dominio y la aplicación los reciben por
//! los puertos `Reloj` y `GeneradorIds`.

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::America::Costa_Rica;
use limen_aplicacion::puertos::{GeneradorIds, Reloj};
use uuid::Uuid;

/// Hora real del sistema. Los instantes se manejan en UTC; las reglas de
/// calendario (vencimiento del PRAIND) usan la fecha de Costa Rica, sin
/// importar la zona horaria configurada en el equipo.
#[derive(Debug, Clone, Copy, Default)]
pub struct RelojCostaRica;

impl RelojCostaRica {
    /// Fecha de Costa Rica que corresponde a un instante UTC.
    pub fn fecha_en_costa_rica(instante: DateTime<Utc>) -> NaiveDate {
        instante.with_timezone(&Costa_Rica).date_naive()
    }
}

impl Reloj for RelojCostaRica {
    fn ahora(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn hoy(&self) -> NaiveDate {
        Self::fecha_en_costa_rica(self.ahora())
    }

    fn inicio_del_dia(&self, fecha: NaiveDate) -> DateTime<Utc> {
        let medianoche = fecha.and_time(NaiveTime::MIN);
        Costa_Rica
            .from_local_datetime(&medianoche)
            .earliest()
            .map_or_else(
                // Costa Rica no tiene horario de verano: nunca falta ni se
                // repite la medianoche. Por si la base de zonas cambiara.
                || Utc.from_utc_datetime(&medianoche),
                |local| local.with_timezone(&Utc),
            )
    }
}

/// IDs UUID v7: únicos entre equipos y ordenados por el momento en que se
/// crean (dentro de un mismo proceso, estrictamente crecientes).
#[derive(Debug, Clone, Copy, Default)]
pub struct IdsV7;

impl GeneradorIds for IdsV7 {
    fn nuevo(&self) -> Uuid {
        Uuid::now_v7()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn utc(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    #[test]
    fn el_dia_de_costa_rica_empieza_a_las_seis_de_la_manana_utc() {
        let dia = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        assert_eq!(
            RelojCostaRica.inicio_del_dia(dia),
            utc("2026-10-09T06:00:00Z")
        );
        // Y el día siguiente empieza 24 horas después (sin horario de verano).
        assert_eq!(
            RelojCostaRica.inicio_del_dia(dia.succ_opt().unwrap()),
            utc("2026-10-10T06:00:00Z")
        );
        // El instante recién antes del inicio todavía es el día anterior.
        assert_eq!(
            RelojCostaRica::fecha_en_costa_rica(utc("2026-10-09T05:59:59Z")),
            dia.pred_opt().unwrap()
        );
    }

    #[test]
    fn costa_rica_va_seis_horas_detras_de_utc() {
        // 05:59 UTC todavía es el día anterior en Costa Rica (23:59).
        assert_eq!(
            RelojCostaRica::fecha_en_costa_rica(utc("2026-10-10T05:59:00Z")),
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
            "antes de las 06:00 UTC sigue siendo ayer"
        );
        assert_eq!(
            RelojCostaRica::fecha_en_costa_rica(utc("2026-10-10T06:00:00Z")),
            NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
            "desde las 06:00 UTC ya es hoy"
        );
    }

    #[test]
    fn el_reloj_real_es_coherente_consigo_mismo() {
        let reloj = RelojCostaRica;
        let ahora = reloj.ahora();
        let hoy = reloj.hoy();
        let esperada = RelojCostaRica::fecha_en_costa_rica(ahora);
        assert!(
            hoy == esperada || hoy == esperada.succ_opt().unwrap(),
            "hoy sale del mismo reloj (o del instante siguiente a medianoche)"
        );
        assert!(
            ahora > Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "el reloj del sistema no está en el pasado remoto"
        );
    }

    #[test]
    fn los_ids_v7_son_crecientes_y_no_se_repiten() {
        let ids = IdsV7;
        let generados: Vec<Uuid> = (0..10_000).map(|_| ids.nuevo()).collect();
        assert!(
            generados.windows(2).all(|par| par[0] < par[1]),
            "cada ID es mayor que el anterior: ordenan por creación"
        );
        assert!(
            generados.iter().all(|id| id.get_version_num() == 7),
            "todos son versión 7"
        );
    }
}
