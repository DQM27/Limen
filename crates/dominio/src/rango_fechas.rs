//! Rangos de fechas para consultar el historial, y los accesos rápidos de
//! siempre (hoy, esta semana, los últimos 30 días…).
//!
//! Son reglas de calendario, así que viven aquí y no en la pantalla: la
//! semana empieza el lunes y "hoy" es el de Costa Rica (el caso de uso lo
//! recibe del reloj). Un extremo vacío está abierto: "desde siempre" o "hasta
//! hoy y lo que venga".

use chrono::{Datelike, Days, Months, NaiveDate};

/// Cuántos meses hacia atrás abre el historial por omisión.
pub const MESES_POR_OMISION: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorRango {
    #[error("La fecha inicial no puede ser posterior a la final")]
    Invertido,
}

impl ErrorRango {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Invertido => "rango_invertido",
        }
    }
}

/// Desde y hasta, ambos días incluidos. Un extremo en `None` está abierto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangoFechas {
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
}

impl RangoFechas {
    /// Sin límites: todo el historial.
    pub const fn abierto() -> Self {
        Self {
            desde: None,
            hasta: None,
        }
    }

    /// El inicio no puede ser posterior al fin.
    pub fn nuevo(desde: Option<NaiveDate>, hasta: Option<NaiveDate>) -> Result<Self, ErrorRango> {
        match (desde, hasta) {
            (Some(desde), Some(hasta)) if desde > hasta => Err(ErrorRango::Invertido),
            _ => Ok(Self { desde, hasta }),
        }
    }

    pub const fn desde(&self) -> Option<NaiveDate> {
        self.desde
    }

    pub const fn hasta(&self) -> Option<NaiveDate> {
        self.hasta
    }
}

/// Un rango de uso frecuente, con su nombre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Atajo {
    Hoy,
    Ayer,
    EstaSemana,
    SemanaPasada,
    EsteMes,
    MesPasado,
    Ultimos7Dias,
    Ultimos30Dias,
    /// Con el fin abierto, para no perder los movimientos del día en curso.
    Ultimos6Meses,
    Todo,
}

impl Atajo {
    pub const TODOS: [Self; 10] = [
        Self::Hoy,
        Self::Ayer,
        Self::EstaSemana,
        Self::SemanaPasada,
        Self::EsteMes,
        Self::MesPasado,
        Self::Ultimos7Dias,
        Self::Ultimos30Dias,
        Self::Ultimos6Meses,
        Self::Todo,
    ];

    /// El que abre el historial.
    pub const POR_OMISION: Self = Self::Ultimos6Meses;

    /// Código estable, para que la interfaz no compare textos.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Hoy => "HOY",
            Self::Ayer => "AYER",
            Self::EstaSemana => "ESTA_SEMANA",
            Self::SemanaPasada => "SEMANA_PASADA",
            Self::EsteMes => "ESTE_MES",
            Self::MesPasado => "MES_PASADO",
            Self::Ultimos7Dias => "ULTIMOS_7_DIAS",
            Self::Ultimos30Dias => "ULTIMOS_30_DIAS",
            Self::Ultimos6Meses => "ULTIMOS_6_MESES",
            Self::Todo => "TODO",
        }
    }

    pub const fn etiqueta(self) -> &'static str {
        match self {
            Self::Hoy => "Hoy",
            Self::Ayer => "Ayer",
            Self::EstaSemana => "Esta semana",
            Self::SemanaPasada => "Semana pasada",
            Self::EsteMes => "Este mes",
            Self::MesPasado => "Mes pasado",
            Self::Ultimos7Dias => "Últimos 7 días",
            Self::Ultimos30Dias => "Últimos 30 días",
            Self::Ultimos6Meses => "Últimos 6 meses",
            Self::Todo => "Todo el historial",
        }
    }

    /// El nombre corto, para el botón de la barra de herramientas.
    pub const fn corta(self) -> &'static str {
        match self {
            Self::Hoy => "Hoy",
            Self::Ayer => "Ayer",
            Self::EstaSemana => "Semana",
            Self::SemanaPasada => "Sem. pasada",
            Self::EsteMes => "Mes",
            Self::MesPasado => "Mes pasado",
            Self::Ultimos7Dias => "7 días",
            Self::Ultimos30Dias => "30 días",
            Self::Ultimos6Meses => "6 meses",
            Self::Todo => "Todo",
        }
    }

    /// El rango que corresponde a este atajo si hoy es `hoy`.
    pub fn rango(self, hoy: NaiveDate) -> RangoFechas {
        let atras = |dias: u64| hoy.checked_sub_days(Days::new(dias)).unwrap_or(hoy);
        let dia = |desde: NaiveDate, hasta: NaiveDate| RangoFechas {
            desde: Some(desde),
            hasta: Some(hasta),
        };
        // El lunes de la semana de `hoy`: la semana laboral arranca el lunes.
        let lunes = atras(u64::from(hoy.weekday().num_days_from_monday()));
        let primero_del_mes = hoy.with_day(1).unwrap_or(hoy);
        match self {
            Self::Hoy => dia(hoy, hoy),
            Self::Ayer => dia(atras(1), atras(1)),
            Self::EstaSemana => dia(lunes, hoy),
            Self::SemanaPasada => dia(
                lunes.checked_sub_days(Days::new(7)).unwrap_or(lunes),
                lunes.checked_sub_days(Days::new(1)).unwrap_or(lunes),
            ),
            Self::EsteMes => dia(primero_del_mes, hoy),
            Self::MesPasado => dia(
                primero_del_mes
                    .checked_sub_months(Months::new(1))
                    .unwrap_or(primero_del_mes),
                primero_del_mes
                    .checked_sub_days(Days::new(1))
                    .unwrap_or(primero_del_mes),
            ),
            Self::Ultimos7Dias => dia(atras(6), hoy),
            Self::Ultimos30Dias => dia(atras(29), hoy),
            Self::Ultimos6Meses => RangoFechas {
                desde: hoy.checked_sub_months(Months::new(MESES_POR_OMISION)),
                hasta: None,
            },
            Self::Todo => RangoFechas::abierto(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    fn rango(atajo: Atajo, hoy: &str) -> (Option<String>, Option<String>) {
        let rango = atajo.rango(fecha(hoy));
        (
            rango.desde().map(|f| f.to_string()),
            rango.hasta().map(|f| f.to_string()),
        )
    }

    fn par(desde: &str, hasta: &str) -> (Option<String>, Option<String>) {
        (Some(desde.into()), Some(hasta.into()))
    }

    #[test]
    fn el_inicio_no_puede_ser_posterior_al_fin() {
        let (a, b) = (fecha("2026-10-01"), fecha("2026-10-09"));
        assert!(RangoFechas::nuevo(Some(a), Some(b)).is_ok());
        assert!(RangoFechas::nuevo(Some(a), Some(a)).is_ok(), "un solo día");
        assert_eq!(
            RangoFechas::nuevo(Some(b), Some(a)),
            Err(ErrorRango::Invertido)
        );
        assert_eq!(ErrorRango::Invertido.codigo(), "rango_invertido");
    }

    #[test]
    fn un_extremo_vacio_esta_abierto() {
        let a = fecha("2026-10-01");
        assert!(RangoFechas::nuevo(Some(a), None).is_ok());
        assert!(RangoFechas::nuevo(None, Some(a)).is_ok());
        assert_eq!(RangoFechas::abierto().desde(), None);
        assert_eq!(RangoFechas::abierto().hasta(), None);
    }

    #[test]
    fn los_atajos_un_viernes() {
        // 9 de octubre de 2026 es viernes.
        assert_eq!(
            rango(Atajo::Hoy, "2026-10-09"),
            par("2026-10-09", "2026-10-09")
        );
        assert_eq!(
            rango(Atajo::Ayer, "2026-10-09"),
            par("2026-10-08", "2026-10-08")
        );
        assert_eq!(
            rango(Atajo::EstaSemana, "2026-10-09"),
            par("2026-10-05", "2026-10-09"),
            "desde el lunes"
        );
        assert_eq!(
            rango(Atajo::SemanaPasada, "2026-10-09"),
            par("2026-09-28", "2026-10-04"),
            "de lunes a domingo"
        );
        assert_eq!(
            rango(Atajo::EsteMes, "2026-10-09"),
            par("2026-10-01", "2026-10-09")
        );
        assert_eq!(
            rango(Atajo::MesPasado, "2026-10-09"),
            par("2026-09-01", "2026-09-30")
        );
        assert_eq!(
            rango(Atajo::Ultimos7Dias, "2026-10-09"),
            par("2026-10-03", "2026-10-09"),
            "hoy cuenta como uno de los siete"
        );
        assert_eq!(
            rango(Atajo::Ultimos30Dias, "2026-10-09"),
            par("2026-09-10", "2026-10-09")
        );
    }

    #[test]
    fn los_seis_meses_dejan_el_fin_abierto_y_todo_deja_ambos() {
        assert_eq!(
            rango(Atajo::Ultimos6Meses, "2026-10-09"),
            (Some("2026-04-09".into()), None)
        );
        assert_eq!(rango(Atajo::Todo, "2026-10-09"), (None, None));
    }

    #[test]
    fn el_domingo_todavia_es_de_la_semana_que_empezo_el_lunes() {
        // 11 de octubre de 2026 es domingo.
        assert_eq!(
            rango(Atajo::EstaSemana, "2026-10-11"),
            par("2026-10-05", "2026-10-11")
        );
        // Y un lunes la semana empieza hoy.
        assert_eq!(
            rango(Atajo::EstaSemana, "2026-10-05"),
            par("2026-10-05", "2026-10-05")
        );
        assert_eq!(
            rango(Atajo::SemanaPasada, "2026-10-05"),
            par("2026-09-28", "2026-10-04")
        );
    }

    #[test]
    fn en_enero_el_mes_pasado_es_diciembre_del_anio_anterior() {
        assert_eq!(
            rango(Atajo::MesPasado, "2026-01-15"),
            par("2025-12-01", "2025-12-31")
        );
        assert_eq!(
            rango(Atajo::SemanaPasada, "2026-01-02"),
            par("2025-12-22", "2025-12-28")
        );
    }

    #[test]
    fn los_seis_meses_a_fin_de_mes_no_se_pasan_de_dia() {
        // 6 meses antes del 31 de agosto: febrero no tiene 31 → el 28.
        assert_eq!(
            rango(Atajo::Ultimos6Meses, "2026-08-31"),
            (Some("2026-02-28".into()), None)
        );
    }

    #[test]
    fn cada_atajo_tiene_codigo_y_nombres_distintos_y_el_de_omision_es_seis_meses() {
        let codigos: std::collections::HashSet<_> =
            Atajo::TODOS.iter().map(|a| a.codigo()).collect();
        assert_eq!(codigos.len(), Atajo::TODOS.len());
        assert!(
            Atajo::TODOS
                .iter()
                .all(|a| !a.etiqueta().is_empty() && !a.corta().is_empty())
        );
        assert_eq!(Atajo::POR_OMISION, Atajo::Ultimos6Meses);
    }

    #[test]
    fn ningun_atajo_produce_un_rango_invertido() {
        for hoy in [
            "2026-01-01",
            "2026-02-28",
            "2026-10-09",
            "2026-12-31",
            "2028-02-29",
        ] {
            for atajo in Atajo::TODOS {
                let r = atajo.rango(fecha(hoy));
                assert!(
                    RangoFechas::nuevo(r.desde(), r.hasta()).is_ok(),
                    "{atajo:?} con hoy = {hoy}"
                );
            }
        }
    }
}
