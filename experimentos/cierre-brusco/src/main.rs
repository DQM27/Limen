//! EXPERIMENTO (rama `exp/rocksdb`): cierre brusco del proceso.
//!
//! - `hijo <motor> <ruta> <filas>`: abre la base y confirma transacciones de
//!   `<filas>` hechos, una tras otra; tras cada confirmación imprime
//!   `OK <total de filas confirmadas>`.
//! - `contar <motor> <ruta>`: abre la base y dice cuántos hechos hay.
//! - `padre <motor> <filas> <repeticiones> <max_ms>`: por repetición lanza un
//!   hijo, lo mata con `taskkill /F` a un instante al azar, y lanza `contar`.
//!   Imprime una línea CSV por repetición.
//!
//! Matar el proceso NO es cortar la energía: el sistema operativo conserva lo
//! que el proceso ya le entregó aunque no esté en el disco.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{FabricaUnidadDeTrabajo, RegistroHechos, UnidadDeTrabajo};
use limen_dominio::hecho::{CalidadHora, Hecho, HechoId, Salida, Suceso};
use limen_dominio::movimiento::Marca;
use limen_dominio::operador::OperadorId;
use limen_dominio::presencia::Via;
use limen_infra_surreal::AlmacenSurreal;
use uuid::Uuid;

fn en() -> DateTime<Utc> {
    "2026-10-09T17:00:00Z".parse().unwrap()
}

async fn abrir(motor: &str, ruta: &Path) -> AlmacenSurreal {
    match motor {
        "surrealkv" => AlmacenSurreal::en_disco(ruta).await.unwrap(),
        #[cfg(feature = "rocksdb")]
        "rocksdb" => AlmacenSurreal::en_disco_rocksdb(ruta).await.unwrap(),
        otro => panic!("motor desconocido: {otro}"),
    }
}

fn hecho(n: u128) -> Hecho {
    Hecho::restaurar(
        HechoId::desde_uuid(Uuid::from_u128(n)),
        Suceso::Salida(Salida {
            via: Via::Contratista,
            registro: Uuid::from_u128(n),
            marca: Marca {
                en: en(),
                operador: OperadorId::desde_uuid(Uuid::from_u128(9)),
            },
        }),
        CalidadHora {
            confiable: true,
            hora_equipo: en(),
        },
    )
}

async fn hijo(motor: &str, ruta: &Path, filas: u128) {
    let almacen = abrir(motor, ruta).await;
    println!("LISTO");
    let mut siguiente: u128 = 1;
    loop {
        let mut uow = almacen.nueva();
        for _ in 0..filas {
            uow.hechos().anotar(hecho(siguiente));
            siguiente += 1;
        }
        uow.confirmar().await.unwrap();
        println!("OK {}", siguiente - 1);
    }
}

async fn contar(motor: &str, ruta: &Path) {
    let t = Instant::now();
    let almacen = abrir(motor, ruta).await;
    let n = almacen.contar_hechos().await.unwrap();
    println!("CONTADOS {n} {}", t.elapsed().as_millis());
}

/// Número pseudoaleatorio sencillo (xorshift), suficiente para variar el instante.
fn azar(estado: &mut u64) -> u64 {
    *estado ^= *estado << 13;
    *estado ^= *estado >> 7;
    *estado ^= *estado << 17;
    *estado
}

fn padre(motor: &str, filas: u128, repeticiones: u32, max_ms: u64) {
    let yo = std::env::current_exe().unwrap();
    let mut estado = 0x9E37_79B9_7F4A_7C15_u64 ^ u64::from(std::process::id());
    println!("motor,filas_por_tx,rep,kill_ms,confirmadas,encontradas,perdidas,reapertura_ms,error");
    for rep in 1..=repeticiones {
        let ruta: PathBuf = std::env::temp_dir().join(format!("limen-cierre-{motor}-{rep}"));
        drop(std::fs::remove_dir_all(&ruta));
        let mut hijo = Command::new(&yo)
            .args(["hijo", motor, ruta.to_str().unwrap(), &filas.to_string()])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let confirmadas = Arc::new(AtomicU64::new(0));
        let listo = Arc::new(AtomicU64::new(0));
        let lector = {
            let (c, l) = (confirmadas.clone(), listo.clone());
            let salida = hijo.stdout.take().unwrap();
            std::thread::spawn(move || {
                for linea in BufReader::new(salida).lines().map_while(Result::ok) {
                    if linea == "LISTO" {
                        l.store(1, Ordering::SeqCst);
                    } else if let Some(n) = linea.strip_prefix("OK ") {
                        c.store(n.parse().unwrap_or(0), Ordering::SeqCst);
                    }
                }
            })
        };
        while listo.load(Ordering::SeqCst) == 0 {
            std::thread::sleep(Duration::from_millis(5));
        }
        let espera = 50 + azar(&mut estado) % max_ms;
        std::thread::sleep(Duration::from_millis(espera));
        // Windows: `taskkill /F` (TerminateProcess). Linux: `kill -9` (SIGKILL).
        let mut orden = if cfg!(windows) {
            let mut c = Command::new("taskkill");
            c.args(["/F", "/PID", &hijo.id().to_string()]);
            c
        } else {
            let mut c = Command::new("kill");
            c.args(["-9", &hijo.id().to_string()]);
            c
        };
        let matar = orden
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(matar.success(), "taskkill falló");
        hijo.wait().unwrap();
        lector.join().unwrap();
        let confirmadas = confirmadas.load(Ordering::SeqCst);

        let salida = Command::new(&yo)
            .args(["contar", motor, ruta.to_str().unwrap()])
            .output()
            .unwrap();
        let texto = String::from_utf8_lossy(&salida.stdout);
        let (encontradas, reapertura, error) = texto
            .lines()
            .find_map(|l| l.strip_prefix("CONTADOS "))
            .and_then(|r| {
                let mut p = r.split(' ');
                Some((
                    p.next()?.parse::<u64>().ok()?,
                    p.next()?.parse::<u64>().ok()?,
                    String::new(),
                ))
            })
            .unwrap_or_else(|| {
                let e = String::from_utf8_lossy(&salida.stderr).replace(['\n', '\r', ','], " ");
                (0, 0, e.chars().take(200).collect())
            });
        let perdidas = confirmadas.saturating_sub(encontradas);
        println!(
            "{motor},{filas},{rep},{espera},{confirmadas},{encontradas},{perdidas},{reapertura},{error}"
        );
        drop(std::fs::remove_dir_all(&ruta));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let a = |i: usize| args.get(i).map(String::as_str).unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    match a(1) {
        "hijo" => rt.block_on(hijo(a(2), Path::new(a(3)), a(4).parse().unwrap())),
        "contar" => rt.block_on(contar(a(2), Path::new(a(3)))),
        "padre" => padre(
            a(2),
            a(3).parse().unwrap(),
            a(4).parse().unwrap(),
            a(5).parse().unwrap(),
        ),
        otro => panic!("modo desconocido: {otro}"),
    }
}
