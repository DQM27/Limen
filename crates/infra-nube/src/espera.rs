//! Cuánto esperar entre reintentos, con variación aleatoria (*jitter*).
//!
//! Si la nube se cae y miles de equipos reintentan con el mismo intervalo,
//! todos vuelven al mismo instante y la nube, que intenta recuperarse, recibe
//! todo junto (*estampida*). Aquí cada equipo espera un tiempo distinto:
//!
//! - [`con_jitter`]: retroceso exponencial con *full jitter*. El techo se
//!   duplica en cada intento fallido (hasta [`TOPE`]) y la espera es un valor
//!   al azar entre [`MINIMA`] y ese techo.
//! - [`variar`]: un periodo fijo, repartido ±[`VARIACION_PORCIENTO`] % para que
//!   equipos arrancados juntos no queden sincronizados entre sí.
//!
//! El azar entra por [`Azar`] para que las pruebas usen uno determinista. Esto
//! es infraestructura: el dominio no genera números al azar. **No sirve para
//! claves, sales ni nonces**: eso sale del generador del sistema operativo.

use std::time::Duration;

use rand::Rng;

/// Lo más que se espera entre dos reintentos.
pub const TOPE: Duration = Duration::from_secs(5 * 60);
/// Lo menos que se espera: evita reintentar en un ciclo apretado si el azar
/// da cero varias veces seguidas.
pub const MINIMA: Duration = Duration::from_secs(1);
/// Cuánto se aparta [`variar`] del periodo que recibe, hacia cada lado.
pub const VARIACION_PORCIENTO: u64 = 15;
/// A partir de aquí el techo ya no se duplica: [`TOPE`] lo alcanza mucho antes
/// y se evita que la potencia de dos se desborde.
const DUPLICACIONES_MAXIMAS: u32 = 16;

/// De dónde sale el azar.
pub trait Azar: Send {
    /// Un número entre 0 y `maximo`, ambos incluidos.
    fn hasta(&mut self, maximo: u64) -> u64;
}

/// El azar de verdad, del generador del hilo actual.
#[derive(Debug, Clone, Copy, Default)]
pub struct AzarDelSistema;

impl Azar for AzarDelSistema {
    fn hasta(&mut self, maximo: u64) -> u64 {
        rand::rng().random_range(0..=maximo)
    }
}

/// El techo de la espera del intento número `intento` (el primero es el 1):
/// `base`, `2 × base`, `4 × base`… sin pasar de [`TOPE`].
pub fn techo(base: Duration, intento: u32) -> Duration {
    let duplicaciones = intento.saturating_sub(1).min(DUPLICACIONES_MAXIMAS);
    base.saturating_mul(1 << duplicaciones).min(TOPE)
}

/// La espera antes del reintento número `intento`: al azar entre [`MINIMA`] y
/// el [`techo`] de ese intento (*full jitter*).
pub fn con_jitter(base: Duration, intento: u32, azar: &mut impl Azar) -> Duration {
    let maximo = milisegundos(techo(base, intento));
    Duration::from_millis(azar.hasta(maximo)).max(MINIMA)
}

/// `periodo` movido al azar hasta ±[`VARIACION_PORCIENTO`] %.
pub fn variar(periodo: Duration, azar: &mut impl Azar) -> Duration {
    let ms = milisegundos(periodo);
    let ancho = ms.saturating_mul(VARIACION_PORCIENTO) / 100;
    let desde = ms.saturating_sub(ancho);
    Duration::from_millis(desde.saturating_add(azar.hasta(ancho.saturating_mul(2))))
}

fn milisegundos(duracion: Duration) -> u64 {
    u64::try_from(duracion.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: Duration = Duration::from_secs(30);

    /// Un azar fijo: da el valor que se le indicó, recortado al máximo pedido.
    struct Fijo(u64);

    impl Azar for Fijo {
        fn hasta(&mut self, maximo: u64) -> u64 {
            self.0.min(maximo)
        }
    }

    #[test]
    fn el_techo_se_duplica_en_cada_intento() {
        assert_eq!(techo(BASE, 1), Duration::from_secs(30), "intento 1");
        assert_eq!(techo(BASE, 2), Duration::from_secs(60), "intento 2");
        assert_eq!(techo(BASE, 3), Duration::from_secs(120), "intento 3");
        assert_eq!(techo(BASE, 4), Duration::from_secs(240), "intento 4");
    }

    #[test]
    fn el_techo_se_detiene_en_el_tope() {
        assert_eq!(techo(BASE, 5), TOPE, "480 s se recorta a 300 s");
        assert_eq!(techo(BASE, 50), TOPE, "muchos intentos no desbordan");
        assert_eq!(techo(BASE, u32::MAX), TOPE, "ni el máximo posible");
    }

    #[test]
    fn el_intento_cero_vale_como_el_primero() {
        assert_eq!(techo(BASE, 0), techo(BASE, 1), "no se resta de más");
    }

    #[test]
    fn con_jitter_va_de_la_minima_al_techo() {
        assert_eq!(con_jitter(BASE, 3, &mut Fijo(0)), MINIMA, "azar en cero");
        assert_eq!(
            con_jitter(BASE, 3, &mut Fijo(u64::MAX)),
            Duration::from_secs(120),
            "azar en el máximo da el techo"
        );
        assert_eq!(
            con_jitter(BASE, 3, &mut Fijo(45_000)),
            Duration::from_secs(45),
            "un valor intermedio se respeta"
        );
    }

    #[test]
    fn con_azar_real_siempre_queda_entre_la_minima_y_el_techo() {
        for intento in 1..=8 {
            for _ in 0..200 {
                let espera = con_jitter(BASE, intento, &mut AzarDelSistema);
                assert!(
                    (MINIMA..=techo(BASE, intento)).contains(&espera),
                    "intento {intento}: {espera:?} fuera de rango"
                );
            }
        }
    }

    #[test]
    fn con_azar_real_los_equipos_no_vuelven_todos_juntos() {
        let esperas: std::collections::HashSet<_> = (0..200)
            .map(|_| con_jitter(BASE, 1, &mut AzarDelSistema))
            .collect();
        assert!(
            esperas.len() > 50,
            "200 equipos deberían repartirse en muchos instantes distintos, no en {}",
            esperas.len()
        );
    }

    #[test]
    fn variar_se_mueve_hasta_quince_por_ciento() {
        let diez = Duration::from_secs(10);
        assert_eq!(
            variar(diez, &mut Fijo(0)),
            Duration::from_millis(8_500),
            "el extremo de abajo"
        );
        assert_eq!(
            variar(diez, &mut Fijo(u64::MAX)),
            Duration::from_millis(11_500),
            "el extremo de arriba"
        );
        assert_eq!(
            variar(diez, &mut Fijo(1_500)),
            diez,
            "la mitad del rango es el periodo exacto"
        );
    }

    #[test]
    fn variar_con_azar_real_queda_dentro_del_rango() {
        let diez = Duration::from_secs(10);
        for _ in 0..500 {
            let periodo = variar(diez, &mut AzarDelSistema);
            assert!(
                (Duration::from_millis(8_500)..=Duration::from_millis(11_500)).contains(&periodo),
                "{periodo:?} fuera de ±15 %"
            );
        }
    }

    #[test]
    fn variar_un_periodo_cero_no_falla() {
        assert_eq!(
            variar(Duration::ZERO, &mut AzarDelSistema),
            Duration::ZERO,
            "sin periodo no hay variación"
        );
    }
}
