//! La hora con que se sella cada movimiento (regla E5).
//!
//! El reloj del equipo puede estar mal (pila agotada, sin sincronizar) o
//! alguien puede cambiarlo. Por eso la hora no se toma del equipo a ciegas:
//! la plataforma entrega una [`LecturaReloj`] (la hora que cree buena, su
//! margen de error y la hora cruda del equipo) y el dominio decide con qué
//! hora se sella y si es confiable.
//!
//! E5: la portería nunca se detiene por el reloj. Si la hora quedó antes del
//! último movimiento registrado (el reloj retrocedió), se sella con la hora
//! de ese último movimiento, para que el historial nunca vaya hacia atrás, y
//! la hora queda marcada como no confiable. También es no confiable la que
//! la plataforma no pudo comprobar contra una fuente externa, o cuyo margen
//! de error es mayor que [`MARGEN_MAXIMO_CONFIABLE_MS`]. La marca queda en
//! el hecho, junto con la hora cruda del equipo: un salto entre las dos
//! delata un reloj cambiado a mano.

use chrono::{DateTime, Utc};

/// Hasta cuánto error, en milisegundos, una hora sigue siendo confiable.
pub const MARGEN_MAXIMO_CONFIABLE_MS: u64 = 60_000;

/// Lo que dice el reloj de la plataforma en un instante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LecturaReloj {
    /// La mejor hora que conoce la plataforma (corregida contra una fuente
    /// externa si pudo).
    pub instante: DateTime<Utc>,
    /// Cuánto puede estar equivocada, en milisegundos. `None`: no se pudo
    /// comprobar contra ninguna fuente externa.
    pub margen_ms: Option<u64>,
    /// La hora del reloj del equipo, tal cual.
    pub hora_equipo: DateTime<Utc>,
}

/// La hora con que se sella un movimiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoraSellada {
    pub en: DateTime<Utc>,
    /// Si la hora es confiable. Una hora no confiable no detiene nada: se
    /// registra igual y queda marcada para revisarla.
    pub confiable: bool,
    /// La hora del reloj del equipo en ese momento, para la auditoría.
    pub hora_equipo: DateTime<Utc>,
}

/// E5: la hora con que se sella un movimiento nuevo.
pub fn sellar_hora(lectura: LecturaReloj, ultimo_movimiento: Option<DateTime<Utc>>) -> HoraSellada {
    let comprobada = lectura
        .margen_ms
        .is_some_and(|margen| margen <= MARGEN_MAXIMO_CONFIABLE_MS);
    match ultimo_movimiento {
        Some(ultimo) if lectura.instante < ultimo => HoraSellada {
            en: ultimo,
            confiable: false,
            hora_equipo: lectura.hora_equipo,
        },
        _ => HoraSellada {
            en: lectura.instante,
            confiable: comprobada,
            hora_equipo: lectura.hora_equipo,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    fn lectura(texto: &str, margen_ms: Option<u64>) -> LecturaReloj {
        LecturaReloj {
            instante: instante(texto),
            margen_ms,
            hora_equipo: instante("2026-10-09T15:11:00Z"),
        }
    }

    #[test]
    fn una_hora_comprobada_y_posterior_se_usa_tal_cual() {
        let sellada = sellar_hora(
            lectura("2026-10-09T15:00:01Z", Some(40)),
            Some(instante("2026-10-09T15:00:00Z")),
        );
        assert_eq!(
            sellada,
            HoraSellada {
                en: instante("2026-10-09T15:00:01Z"),
                confiable: true,
                hora_equipo: instante("2026-10-09T15:11:00Z"),
            },
            "con la hora cruda del equipo al lado, aunque esté adelantado"
        );
        assert!(
            sellar_hora(lectura("2020-01-01T00:00:00Z", Some(0)), None).confiable,
            "sin movimientos previos cualquier hora comprobada vale"
        );
    }

    #[test]
    fn si_el_reloj_retrocedio_se_registra_igual_con_la_hora_del_ultimo_movimiento() {
        let ultimo = instante("2026-10-09T15:00:00Z");
        let sellada = sellar_hora(lectura("2026-10-09T14:59:59Z", Some(40)), Some(ultimo));
        assert_eq!(sellada.en, ultimo, "el historial nunca va hacia atrás");
        assert!(!sellada.confiable, "y queda marcada para revisarla");
        assert!(
            sellar_hora(lectura("2026-10-09T15:00:00Z", Some(40)), Some(ultimo)).confiable,
            "la misma hora del último movimiento no es un retroceso"
        );
    }

    #[test]
    fn sin_comprobar_o_con_mucho_margen_no_es_confiable() {
        assert!(
            !sellar_hora(lectura("2026-10-09T15:00:00Z", None), None).confiable,
            "nunca se comprobó contra una fuente externa"
        );
        assert!(
            sellar_hora(
                lectura("2026-10-09T15:00:00Z", Some(MARGEN_MAXIMO_CONFIABLE_MS)),
                None
            )
            .confiable,
            "el margen máximo todavía es confiable"
        );
        assert!(
            !sellar_hora(
                lectura("2026-10-09T15:00:00Z", Some(MARGEN_MAXIMO_CONFIABLE_MS + 1)),
                None
            )
            .confiable,
            "un milisegundo más ya no"
        );
    }
}
