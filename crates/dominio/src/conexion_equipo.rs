//! Cómo está conectado un equipo (panel de equipos).
//!
//! Cada equipo avisa a la nube cuando abre la app, avisa cada cinco minutos que sigue abierto y,
//! si se cierra de forma normal, avisa que se va. Con eso el panel distingue tres estados:
//!
//! - **En línea**: la app está abierta y avisó hace poco.
//! - **Sin señal**: dijo que estaba abierta pero lleva más de [`MINUTOS_SIN_SENAL`] sin avisar, y
//!   no se despidió: se cerró de golpe, el equipo se apagó o perdió internet.
//! - **Desconectado**: la app se cerró de forma normal y avisó que se iba.
//!
//! La hora la pone la nube, así que no depende del reloj del equipo ni de la zona horaria.

use chrono::{DateTime, TimeDelta, Utc};

/// Cuántos minutos sin avisar bastan para dar por perdida la señal de un equipo: dos latidos
/// (cada cinco minutos) y un margen para un aviso que tarde.
pub const MINUTOS_SIN_SENAL: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoConexion {
    EnLinea,
    SinSenal,
    Desconectado,
}

impl EstadoConexion {
    /// Código estable (el que viaja a la página y se guarda en pruebas).
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::EnLinea => "en_linea",
            Self::SinSenal => "caido",
            Self::Desconectado => "fuera",
        }
    }
}

/// El estado de un equipo: `anuncia_en_linea` es lo último que dijo de sí mismo (`true` al abrir
/// y en cada latido, `false` al despedirse), `visto` cuándo fue su última señal y `ahora` la hora
/// de la nube.
pub fn estado_de_conexion(
    anuncia_en_linea: bool,
    visto: DateTime<Utc>,
    ahora: DateTime<Utc>,
) -> EstadoConexion {
    if !anuncia_en_linea {
        return EstadoConexion::Desconectado;
    }
    if ahora - visto > TimeDelta::minutes(MINUTOS_SIN_SENAL) {
        EstadoConexion::SinSenal
    } else {
        EstadoConexion::EnLinea
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    const VISTO: &str = "2026-10-14T14:00:00Z";

    #[test]
    fn avisando_a_tiempo_esta_en_linea() {
        assert_eq!(
            estado_de_conexion(true, en(VISTO), en("2026-10-14T14:04:59Z")),
            EstadoConexion::EnLinea
        );
    }

    #[test]
    fn diez_minutos_exactos_todavia_es_en_linea_y_un_segundo_mas_es_sin_senal() {
        assert_eq!(
            estado_de_conexion(true, en(VISTO), en("2026-10-14T14:10:00Z")),
            EstadoConexion::EnLinea,
            "es «más de» diez minutos"
        );
        assert_eq!(
            estado_de_conexion(true, en(VISTO), en("2026-10-14T14:10:01Z")),
            EstadoConexion::SinSenal
        );
    }

    #[test]
    fn si_se_despidio_esta_desconectado_sin_importar_cuanto_haya_pasado() {
        for ahora in ["2026-10-14T14:00:01Z", "2026-10-15T14:00:00Z"] {
            assert_eq!(
                estado_de_conexion(false, en(VISTO), en(ahora)),
                EstadoConexion::Desconectado,
                "{ahora}"
            );
        }
    }

    #[test]
    fn una_senal_del_futuro_no_hace_caer_al_equipo() {
        assert_eq!(
            estado_de_conexion(true, en("2026-10-14T14:00:30Z"), en(VISTO)),
            EstadoConexion::EnLinea,
            "un reloj adelantado no lo da por perdido"
        );
    }

    #[test]
    fn los_codigos_son_los_de_siempre() {
        assert_eq!(EstadoConexion::EnLinea.codigo(), "en_linea");
        assert_eq!(EstadoConexion::SinSenal.codigo(), "caido");
        assert_eq!(EstadoConexion::Desconectado.codigo(), "fuera");
    }
}
