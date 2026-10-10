//! Medición de la hora contra un servidor NTP (SNTP, RFC 4330), sin
//! dependencias: un paquete UDP de 48 bytes de ida y otro de vuelta.
//!
//! Se pregunta varias veces y se usa la respuesta que tardó MENOS en la red:
//! su error es, como mucho, la mitad de ese viaje. Igual que Lattis contra su
//! servidor, pero con los dos sellos que trae NTP (cuándo recibió y cuándo
//! respondió el servidor), así el tiempo que el servidor tardó en contestar
//! no se cuenta como viaje.
//!
//! Toda la cuenta se hace con el reloj monotónico del proceso (`Instant`),
//! que nadie puede mover: el reloj del equipo no interviene.

use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeDelta, Utc};

/// Cuántas veces se pregunta por servidor.
const MUESTRAS: usize = 4;
/// Tope de espera por respuesta.
const ESPERA: Duration = Duration::from_secs(2);
/// Si ni la mejor muestra bajó de esto, la red está demasiado lenta.
const VIAJE_MAXIMO_UTIL: Duration = Duration::from_millis(1_500);
/// Segundos entre 1900 (época NTP) y 1970 (época Unix).
const SEGUNDOS_1900_A_1970: i64 = 2_208_988_800;
/// Lo que suma la siguiente era NTP (desde febrero de 2036).
const SEGUNDOS_POR_ERA: i64 = 1 << 32;
const LARGO_PAQUETE: usize = 48;

/// Una medición: la hora de la fuente en el instante monotónico `tomada`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Medicion {
    pub hora_fuente: DateTime<Utc>,
    pub tomada: Instant,
    /// Cuánto puede estar equivocada, en milisegundos.
    pub margen_ms: u64,
}

/// La mejor de varias mediciones contra `servidor` (`"host"` o
/// `"host:puerto"`; sin puerto, el 123). `None` si no respondió bien o la
/// red estaba demasiado lenta.
pub fn medir(servidor: &str) -> Option<Medicion> {
    let direccion = if servidor.contains(':') {
        servidor.to_owned()
    } else {
        format!("{servidor}:123")
    };
    let destino = direccion.to_socket_addrs().ok()?.next()?;
    let socket = UdpSocket::bind(if destino.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })
    .ok()?;
    socket.connect(destino).ok()?;
    socket.set_read_timeout(Some(ESPERA)).ok()?;
    (0..MUESTRAS)
        .filter_map(|_| una_muestra(&socket))
        .min_by_key(|(viaje, _)| *viaje)
        .filter(|(viaje, _)| *viaje <= VIAJE_MAXIMO_UTIL)
        .map(|(_, medicion)| medicion)
}

/// Una pregunta y su respuesta: el viaje neto por la red y la medición.
fn una_muestra(socket: &UdpSocket) -> Option<(Duration, Medicion)> {
    // El sello de envío viaja de vuelta en la respuesta: así se descarta una
    // respuesta que no es a esta pregunta. Basta con que sea distinto en
    // cada muestra; no se usa como hora.
    let marca = u64::try_from(Utc::now().timestamp_nanos_opt()?).ok()?;
    let mut pedido = [0_u8; LARGO_PAQUETE];
    // LI = 0, versión 4, modo 3 (cliente).
    *pedido.first_mut()? = 0b0010_0011;
    pedido
        .get_mut(40..48)?
        .copy_from_slice(&marca.to_be_bytes());

    let enviado = Instant::now();
    socket.send(&pedido).ok()?;
    let mut respuesta = [0_u8; LARGO_PAQUETE];
    let leidos = socket.recv(&mut respuesta).ok()?;
    let recibido = Instant::now();
    if leidos < LARGO_PAQUETE {
        return None;
    }
    let respuesta = Respuesta::leer(&respuesta, marca)?;

    // Viaje neto: lo que tardó todo, menos lo que el servidor se demoró
    // entre recibir y responder.
    let total = recibido.duration_since(enviado);
    let en_el_servidor = (respuesta.respondio - respuesta.recibio).to_std().ok()?;
    let viaje = total.checked_sub(en_el_servidor)?;
    // El servidor respondió a mitad del viaje de vuelta: al recibir, su hora
    // era la de su respuesta más medio viaje.
    let medio_viaje = viaje / 2;
    let medicion = Medicion {
        hora_fuente: respuesta.respondio + TimeDelta::from_std(medio_viaje).ok()?,
        tomada: recibido,
        margen_ms: u64::try_from(medio_viaje.as_millis())
            .ok()?
            .saturating_add(1),
    };
    Some((viaje, medicion))
}

/// Lo que importa de una respuesta del servidor.
struct Respuesta {
    recibio: DateTime<Utc>,
    respondio: DateTime<Utc>,
}

impl Respuesta {
    /// Valida y lee una respuesta. `marca` es el sello que mandó el cliente.
    fn leer(paquete: &[u8; LARGO_PAQUETE], marca: u64) -> Option<Self> {
        let primero = *paquete.first()?;
        let modo = primero & 0b111;
        let salto = primero >> 6;
        let estrato = *paquete.get(1)?;
        // Modo 4 (servidor), sin aviso de "reloj sin sincronizar" (LI = 3) y
        // con un estrato válido (0 es un "beso de la muerte": no usar).
        if modo != 4 || salto == 3 || !(1..=15).contains(&estrato) {
            return None;
        }
        if leer_u64(paquete, 24)? != marca {
            return None;
        }
        Some(Self {
            recibio: sello_a_hora(leer_u64(paquete, 32)?)?,
            respondio: sello_a_hora(leer_u64(paquete, 40)?)?,
        })
    }
}

fn leer_u64(paquete: &[u8; LARGO_PAQUETE], desde: usize) -> Option<u64> {
    let bytes: [u8; 8] = paquete.get(desde..desde + 8)?.try_into().ok()?;
    Some(u64::from_be_bytes(bytes))
}

/// Un sello NTP (segundos desde 1900 y fracción de segundo) como hora UTC.
fn sello_a_hora(sello: u64) -> Option<DateTime<Utc>> {
    let segundos = i64::try_from(sello >> 32).ok()?;
    if segundos == 0 {
        return None;
    }
    // Un sello con el bit alto apagado es de la era siguiente (2036 en
    // adelante): el contador de 32 bits volvió a empezar.
    let segundos = if segundos < (1 << 31) {
        segundos + SEGUNDOS_POR_ERA
    } else {
        segundos
    };
    let fraccion = u128::from(sello & 0xFFFF_FFFF);
    let nanos = u32::try_from((fraccion * 1_000_000_000) >> 32).ok()?;
    DateTime::from_timestamp(segundos - SEGUNDOS_1900_A_1970, nanos)
}

#[cfg(test)]
pub mod pruebas {
    use std::thread;

    use super::*;

    /// Un sello NTP para una hora UTC (sólo la era actual).
    pub fn hora_a_sello(hora: DateTime<Utc>) -> u64 {
        let segundos = u64::try_from(hora.timestamp() + SEGUNDOS_1900_A_1970).unwrap();
        let fraccion = (u64::from(hora.timestamp_subsec_nanos()) << 32) / 1_000_000_000;
        (segundos << 32) | fraccion
    }

    /// Un servidor NTP de mentira en localhost: responde `veces` preguntas
    /// con la hora real corrida `adelanto` y el formato que se le indique.
    pub fn servidor(
        veces: usize,
        adelanto: TimeDelta,
        retocar: fn(&mut [u8; LARGO_PAQUETE]),
    ) -> String {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let direccion = socket.local_addr().unwrap();
        thread::spawn(move || {
            for _ in 0..veces {
                let mut pedido = [0_u8; LARGO_PAQUETE];
                let Ok((_, cliente)) = socket.recv_from(&mut pedido) else {
                    return;
                };
                let ahora = hora_a_sello(Utc::now() + adelanto).to_be_bytes();
                let mut respuesta = [0_u8; LARGO_PAQUETE];
                respuesta[0] = 0b0010_0100; // versión 4, modo 4 (servidor)
                respuesta[1] = 2; // estrato 2
                respuesta[24..32].copy_from_slice(&pedido[40..48]);
                respuesta[32..40].copy_from_slice(&ahora);
                respuesta[40..48].copy_from_slice(&ahora);
                retocar(&mut respuesta);
                if socket.send_to(&respuesta, cliente).is_err() {
                    return;
                }
            }
        });
        direccion.to_string()
    }

    #[test]
    fn mide_la_hora_de_un_servidor_adelantado() {
        let adelanto = TimeDelta::seconds(2_345);
        let servidor = servidor(MUESTRAS, adelanto, |_| {});
        let medicion = medir(&servidor).expect("responde");
        let esperada = Utc::now() + adelanto;
        let diferencia = (medicion.hora_fuente - esperada).num_milliseconds().abs();
        assert!(diferencia < 500, "medida a {diferencia} ms de la real");
        assert!(medicion.margen_ms < 500, "en localhost el margen es chico");
    }

    #[test]
    fn descarta_respuestas_invalidas() {
        let sin_sincronizar = servidor(MUESTRAS, TimeDelta::zero(), |r| r[0] |= 0b1100_0000);
        assert_eq!(
            medir(&sin_sincronizar),
            None,
            "LI = 3: el servidor no sabe la hora"
        );
        let estrato_cero = servidor(MUESTRAS, TimeDelta::zero(), |r| r[1] = 0);
        assert_eq!(medir(&estrato_cero), None, "estrato 0");
        let ajena = servidor(MUESTRAS, TimeDelta::zero(), |r| r[24] ^= 0xFF);
        assert_eq!(medir(&ajena), None, "respuesta a otra pregunta");
    }

    #[test]
    fn sin_servidor_no_hay_medicion() {
        // Un puerto local sin nadie escuchando: no responde.
        let mudo = UdpSocket::bind("127.0.0.1:0").unwrap();
        let direccion = mudo.local_addr().unwrap().to_string();
        drop(mudo);
        assert_eq!(medir(&direccion), None);
    }

    #[test]
    fn convierte_sellos_ntp_de_ida_y_vuelta() {
        let hora: DateTime<Utc> = "2026-10-10T09:30:15.250Z".parse().unwrap();
        let vuelta = sello_a_hora(hora_a_sello(hora)).unwrap();
        assert!(
            (vuelta - hora).num_microseconds().unwrap().abs() < 1,
            "{vuelta}"
        );
        // Después de febrero de 2036 el contador de segundos vuelve a cero.
        let era_siguiente = 10_u64 << 32;
        assert_eq!(
            sello_a_hora(era_siguiente).unwrap(),
            "2036-02-07T06:28:26Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(sello_a_hora(0), None, "un sello vacío no es una hora");
    }
}
