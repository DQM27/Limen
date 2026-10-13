//! Cuánto lleva alguien adentro.
//!
//! Regla de negocio (E13 de `docs/reglas.md`): quien lleva más de 12 horas
//! adentro se marca para revisión, porque casi siempre significa que su
//! salida no se registró. Sólo marca: no cierra el ingreso ni bloquea nada.

use chrono::{DateTime, Duration, Utc};

/// Pasadas estas horas adentro, se marca para revisión.
pub const HORAS_DE_ALERTA: i64 = 12;

/// Si lleva más de [`HORAS_DE_ALERTA`] horas adentro. Exactamente 12 horas
/// todavía no cuenta.
pub fn lleva_demasiado_adentro(entrada: DateTime<Utc>, ahora: DateTime<Utc>) -> bool {
    ahora - entrada > Duration::hours(HORAS_DE_ALERTA)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    #[test]
    fn marca_a_quien_lleva_mas_de_doce_horas() {
        let entrada = instante("2026-10-09T08:00:00Z");
        assert!(!lleva_demasiado_adentro(
            entrada,
            instante("2026-10-09T19:59:59Z")
        ));
        assert!(
            !lleva_demasiado_adentro(entrada, instante("2026-10-09T20:00:00Z")),
            "exactamente doce horas todavía no"
        );
        assert!(lleva_demasiado_adentro(
            entrada,
            instante("2026-10-09T20:00:01Z")
        ));
    }

    #[test]
    fn una_entrada_en_el_futuro_no_marca_a_nadie() {
        let entrada = instante("2026-10-09T08:00:00Z");
        assert!(!lleva_demasiado_adentro(
            entrada,
            instante("2026-10-09T07:00:00Z")
        ));
    }
}
