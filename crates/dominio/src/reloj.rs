//! Regla E5: si el reloj del equipo retrocedió respecto al último
//! movimiento registrado, no se registra nada.
//!
//! Un reloj atrasado (pila agotada, cambio manual de hora) produciría
//! movimientos con hora anterior a otros ya guardados: un historial
//! imposible de auditar. La hora del último movimiento la entrega el caso de
//! uso; el dominio decide.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "El reloj del equipo está atrasado respecto al último movimiento registrado: corrija la hora"
)]
pub struct RelojAtrasado;

impl RelojAtrasado {
    pub const fn codigo(self) -> &'static str {
        "reloj_atrasado"
    }
}

/// `Ok` si `ahora` no es anterior al último movimiento (o si no hay ninguno).
pub fn verificar_reloj(
    ahora: DateTime<Utc>,
    ultimo_movimiento: Option<DateTime<Utc>>,
) -> Result<(), RelojAtrasado> {
    match ultimo_movimiento {
        Some(ultimo) if ahora < ultimo => Err(RelojAtrasado),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    #[test]
    fn sin_movimientos_previos_cualquier_hora_vale() {
        assert_eq!(
            verificar_reloj(instante("2020-01-01T00:00:00Z"), None),
            Ok(())
        );
    }

    #[test]
    fn la_misma_hora_o_una_posterior_vale() {
        let ultimo = instante("2026-10-09T15:00:00Z");
        assert_eq!(
            verificar_reloj(ultimo, Some(ultimo)),
            Ok(()),
            "la misma hora"
        );
        assert_eq!(
            verificar_reloj(instante("2026-10-09T15:00:01Z"), Some(ultimo)),
            Ok(()),
            "un segundo después"
        );
    }

    #[test]
    fn una_hora_anterior_al_ultimo_movimiento_se_rechaza() {
        let ultimo = instante("2026-10-09T15:00:00Z");
        assert_eq!(
            verificar_reloj(instante("2026-10-09T14:59:59Z"), Some(ultimo)),
            Err(RelojAtrasado)
        );
    }
}
