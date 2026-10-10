//! El reloj de la app: la hora de una fuente externa (NTP hoy; la nube
//! cuando exista), anclada al reloj monotónico del proceso.
//!
//! Cada medición fija un **ancla**: "la fuente decía tal hora en tal
//! instante del reloj monotónico". Desde ahí la hora es *ancla + tiempo
//! transcurrido en el reloj monotónico*, que nadie puede mover: aunque
//! alguien cambie la hora de Windows, los movimientos se sellan bien. Es el
//! patrón de `Kronos` (Lyft) y `TrustedTime` (Google), el mismo de Lattis.
//!
//! Sin ancla (la app recién abrió y todavía no midió, o no hay red) se usa
//! el reloj del equipo corregido con el último desfase medido (que se guarda
//! entre aperturas), y la hora sale **sin comprobar**: el dominio la marca
//! como no confiable (regla E5). Nada se detiene.
//!
//! A diferencia de Lattis, el ancla no sobrevive a cerrar la app: el reloj
//! monotónico de Rust es del proceso (leer el contador de arranque del
//! sistema exige código inseguro, prohibido en el proyecto). Al abrir se
//! vuelve a medir, que tarda menos de un segundo con red.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use limen_aplicacion::puertos::Reloj;
use limen_dominio::reloj::LecturaReloj;

use crate::RelojCostaRica;
use crate::sntp::{self, Medicion};

/// Servidores NTP públicos, en orden de preferencia. Cuando exista la nube,
/// su hora va primero.
pub const SERVIDORES_NTP: [&str; 2] = ["time.windows.com", "pool.ntp.org"];
/// Cada cuánto se vuelve a medir con éxito (corrige la deriva del ancla).
const CADA_CUANTO: Duration = Duration::from_secs(60 * 60);
/// Cada cuánto se reintenta si no se pudo medir.
const REINTENTO: Duration = Duration::from_secs(5 * 60);

/// Deriva máxima del reloj monotónico: 100 partes por millón (0,01 %),
/// holgado para el cristal de una PC (típicamente 20 a 50). Es lo que crece
/// el margen por cada milisegundo desde el ancla.
const DERIVA_PPM: u64 = 100;

#[derive(Debug, Clone, Copy)]
struct Ancla {
    medicion: Medicion,
    /// Reloj del equipo MENOS hora de la fuente, al anclar (ms).
    desfase_ms: i64,
}

#[derive(Debug, Default)]
struct Estado {
    ancla: Option<Ancla>,
    /// El último desfase conocido (de esta apertura o de la anterior).
    desfase_ms: Option<i64>,
}

/// Reloj confiable. Clonarlo comparte el mismo estado: el hilo que mide y
/// los casos de uso ven la misma ancla.
#[derive(Debug, Clone, Default)]
pub struct RelojConfiable {
    estado: Arc<Mutex<Estado>>,
}

impl RelojConfiable {
    /// Un reloj sin ancla. `desfase_guardado`: el último desfase medido en
    /// una apertura anterior, si lo hay.
    pub fn new(desfase_guardado: Option<i64>) -> Self {
        Self {
            estado: Arc::new(Mutex::new(Estado {
                ancla: None,
                desfase_ms: desfase_guardado,
            })),
        }
    }

    fn estado(&self) -> MutexGuard<'_, Estado> {
        self.estado.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Fija el ancla con una medición recién hecha. `hora_equipo`: lo que
    /// decía el reloj del equipo en el mismo instante. Devuelve el desfase
    /// (reloj del equipo MENOS hora de la fuente, en ms), para guardarlo.
    pub fn anclar(&self, medicion: Medicion, hora_equipo: DateTime<Utc>) -> i64 {
        let desfase_ms = (hora_equipo - medicion.hora_fuente).num_milliseconds();
        let mut estado = self.estado();
        estado.ancla = Some(Ancla {
            medicion,
            desfase_ms,
        });
        estado.desfase_ms = Some(desfase_ms);
        drop(estado);
        desfase_ms
    }

    /// Mide contra el primer servidor que responda y fija el ancla.
    /// Devuelve el desfase medido, o `None` si ninguno respondió.
    pub fn sincronizar(&self, servidores: &[&str]) -> Option<i64> {
        let medicion = servidores
            .iter()
            .find_map(|servidor| sntp::medir(servidor))?;
        // Lo que decía el reloj del equipo en el instante de la medición.
        let desde = Instant::now().saturating_duration_since(medicion.tomada);
        let hora_equipo = Utc::now() - TimeDelta::from_std(desde).ok()?;
        Some(self.anclar(medicion, hora_equipo))
    }

    /// Mide ahora y después cada hora (cada 5 minutos mientras no lo
    /// logre), en un hilo propio, y guarda cada desfase en `ruta` para la
    /// próxima apertura. Nunca detiene a nadie: si no hay red, sigue
    /// intentando.
    pub fn sincronizar_en_segundo_plano(&self, servidores: &'static [&'static str], ruta: PathBuf) {
        let reloj = self.clone();
        let lanzado = thread::Builder::new()
            .name("reloj".to_owned())
            .spawn(move || {
                loop {
                    let espera = reloj.sincronizar(servidores).map_or_else(
                        || {
                            log::warn!("no se pudo medir la hora contra ningún servidor NTP");
                            REINTENTO
                        },
                        |desfase| {
                            if let Err(error) = guardar_desfase(&ruta, desfase) {
                                log::warn!("no se pudo guardar el desfase del reloj: {error}");
                            }
                            CADA_CUANTO
                        },
                    );
                    thread::sleep(espera);
                }
            });
        if let Err(error) = lanzado {
            log::error!("no se pudo lanzar la sincronización del reloj: {error}");
        }
    }

    /// La lectura en un momento dado: `equipo` es el reloj del equipo y
    /// `monotonico`, el reloj monotónico del proceso en ese mismo momento.
    fn lectura_en(&self, equipo: DateTime<Utc>, monotonico: Instant) -> LecturaReloj {
        let estado = self.estado();
        let segun_ancla = estado.ancla.and_then(|ancla| {
            let transcurrido = monotonico.checked_duration_since(ancla.medicion.tomada)?;
            let instante = ancla.medicion.hora_fuente + TimeDelta::from_std(transcurrido).ok()?;
            let ms = u64::try_from(transcurrido.as_millis()).ok()?;
            let margen = ancla
                .medicion
                .margen_ms
                .saturating_add(ms.saturating_mul(DERIVA_PPM) / 1_000_000);
            Some((instante, margen, ancla.desfase_ms))
        });
        let desfase_guardado = estado.desfase_ms;
        drop(estado);
        match segun_ancla {
            Some((instante, margen, desfase_al_anclar)) => {
                avisar_si_cambiaron_la_hora(equipo, instante, desfase_al_anclar);
                LecturaReloj {
                    instante,
                    margen_ms: Some(margen),
                    hora_equipo: equipo,
                }
            }
            None => LecturaReloj {
                instante: desfase_guardado
                    .map_or(equipo, |desfase| equipo - TimeDelta::milliseconds(desfase)),
                margen_ms: None,
                hora_equipo: equipo,
            },
        }
    }
}

/// El desfase guardado en la apertura anterior, si lo hay y se puede leer.
pub fn leer_desfase(ruta: &Path) -> Option<i64> {
    std::fs::read_to_string(ruta).ok()?.trim().parse().ok()
}

/// Guarda el desfase (reloj del equipo MENOS hora de la fuente, en ms).
pub fn guardar_desfase(ruta: &Path, desfase_ms: i64) -> std::io::Result<()> {
    std::fs::write(ruta, desfase_ms.to_string())
}

/// Más de esto entre el desfase al anclar y el de ahora: alguien movió la
/// hora del equipo (o el equipo la corrigió de golpe).
const SALTO_SOSPECHOSO_MS: i64 = 2 * 60 * 1000;

/// Deja constancia en el registro técnico de un salto del reloj del equipo.
/// La evidencia que vale para la auditoría es la `hora_equipo` de cada
/// hecho; esto es para quien mantiene el sistema.
fn avisar_si_cambiaron_la_hora(equipo: DateTime<Utc>, hora: DateTime<Utc>, desfase_al_anclar: i64) {
    let desfase_ahora = (equipo - hora).num_milliseconds();
    if (desfase_ahora - desfase_al_anclar).abs() > SALTO_SOSPECHOSO_MS {
        log::warn!(
            "la hora del equipo saltó: se desfasaba {desfase_al_anclar} ms y ahora {desfase_ahora} ms"
        );
    }
}

impl Reloj for RelojConfiable {
    fn ahora(&self) -> DateTime<Utc> {
        self.lectura().instante
    }

    fn hoy(&self) -> NaiveDate {
        RelojCostaRica::fecha_en_costa_rica(self.ahora())
    }

    fn inicio_del_dia(&self, fecha: NaiveDate) -> DateTime<Utc> {
        RelojCostaRica.inicio_del_dia(fecha)
    }

    fn lectura(&self) -> LecturaReloj {
        self.lectura_en(Utc::now(), Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn hora(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    /// La fuente dice 15:00:00 en el instante `tomada`, con 40 ms de margen.
    fn medicion(tomada: Instant) -> Medicion {
        Medicion {
            hora_fuente: hora("2026-10-10T15:00:00Z"),
            tomada,
            margen_ms: 40,
        }
    }

    #[test]
    fn sin_medir_usa_el_reloj_del_equipo_sin_comprobar() {
        let reloj = RelojConfiable::new(None);
        let equipo = hora("2026-10-10T15:11:00Z");
        let lectura = reloj.lectura_en(equipo, Instant::now());
        assert_eq!(lectura.instante, equipo);
        assert_eq!(lectura.margen_ms, None, "no se pudo comprobar");
        assert_eq!(lectura.hora_equipo, equipo);
    }

    #[test]
    fn sin_medir_corrige_con_el_desfase_de_la_apertura_anterior() {
        // El equipo iba 11 minutos adelantado la última vez.
        let reloj = RelojConfiable::new(Some(11 * 60 * 1000));
        let lectura = reloj.lectura_en(hora("2026-10-10T15:11:00Z"), Instant::now());
        assert_eq!(lectura.instante, hora("2026-10-10T15:00:00Z"));
        assert_eq!(lectura.margen_ms, None, "corregida, pero sin comprobar");
    }

    #[test]
    fn con_ancla_la_hora_sale_del_reloj_monotonico_aunque_cambien_la_de_windows() {
        let reloj = RelojConfiable::new(None);
        let tomada = Instant::now();
        let desfase = reloj.anclar(medicion(tomada), hora("2026-10-10T15:11:00Z"));
        assert_eq!(
            desfase,
            11 * 60 * 1000,
            "el equipo va 11 minutos adelantado"
        );

        // Diez minutos después alguien pone el reloj de Windows en 1999.
        let despues = tomada + Duration::from_secs(600);
        let lectura = reloj.lectura_en(hora("1999-01-01T00:00:00Z"), despues);
        assert_eq!(
            lectura.instante,
            hora("2026-10-10T15:10:00Z"),
            "la hora sigue bien"
        );
        assert_eq!(
            lectura.hora_equipo,
            hora("1999-01-01T00:00:00Z"),
            "y la del equipo queda como evidencia"
        );
    }

    #[test]
    fn el_margen_crece_con_la_deriva_del_reloj_monotonico() {
        let reloj = RelojConfiable::new(None);
        let tomada = Instant::now();
        reloj.anclar(medicion(tomada), hora("2026-10-10T15:00:00Z"));
        let al_anclar = reloj.lectura_en(hora("2026-10-10T15:00:00Z"), tomada);
        assert_eq!(al_anclar.margen_ms, Some(40));
        // Un día después: 86 400 000 ms × 100 ppm = 8 640 ms más.
        let un_dia = tomada + Duration::from_secs(86_400);
        let lectura = reloj.lectura_en(hora("2026-10-11T15:00:00Z"), un_dia);
        assert_eq!(lectura.margen_ms, Some(40 + 8_640));
    }

    #[test]
    fn sincroniza_contra_el_primer_servidor_que_responde() {
        let adelantado = sntp::pruebas::servidor(4, TimeDelta::seconds(90), |_| {});
        let mudo = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let sin_respuesta = mudo.local_addr().unwrap().to_string();
        drop(mudo);
        let reloj = RelojConfiable::new(None);
        let desfase = reloj
            .sincronizar(&[sin_respuesta.as_str(), adelantado.as_str()])
            .expect("el segundo responde");
        assert!(
            (desfase + 90_000).abs() < 500,
            "la fuente va 90 s adelantada respecto del equipo: {desfase}"
        );
        assert!(reloj.lectura().margen_ms.is_some(), "ya está comprobada");
        assert_eq!(RelojConfiable::new(None).sincronizar(&[]), None);
    }

    #[test]
    fn el_desfase_se_guarda_para_la_proxima_apertura() {
        let ruta = std::env::temp_dir().join(format!("limen-reloj-{}", uuid::Uuid::now_v7()));
        assert_eq!(leer_desfase(&ruta), None, "nunca se midió");
        guardar_desfase(&ruta, -1_234).unwrap();
        assert_eq!(leer_desfase(&ruta), Some(-1_234));
        std::fs::write(&ruta, "no es un número").unwrap();
        assert_eq!(leer_desfase(&ruta), None, "un archivo dañado se ignora");
        std::fs::remove_file(&ruta).unwrap();
    }

    #[test]
    fn los_clones_comparten_el_ancla() {
        let reloj = RelojConfiable::new(None);
        let otro = reloj.clone();
        let tomada = Instant::now();
        otro.anclar(medicion(tomada), hora("2026-10-10T15:00:00Z"));
        assert!(
            reloj
                .lectura_en(hora("2026-10-10T15:00:00Z"), tomada)
                .margen_ms
                .is_some(),
            "el hilo que mide y los casos de uso ven la misma ancla"
        );
    }
}
