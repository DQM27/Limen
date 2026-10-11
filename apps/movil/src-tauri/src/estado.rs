//! Lo que Tauri comparte entre todos los comandos.
//!
//! A diferencia del escritorio, aquí la base se abre en segundo plano (en el
//! teléfono recuperar la base tarda y, si se hiciera en el hilo principal,
//! Android mostraría "la app no responde"). Los comandos esperan a que esté
//! lista; son `async`, así que sólo se detiene su hilo de trabajo.

use std::sync::OnceLock;

use limen_composicion::AplicacionLimen;
use limen_escritorio_comandos::Comandos;
use limen_infra_plataforma::{ClavesArgon2, IdsV7, RelojConfiable};
use limen_infra_surreal::AlmacenSurreal;

/// Los comandos con los adaptadores reales.
pub type ComandosLimen = Comandos<AlmacenSurreal, RelojConfiable, IdsV7, ClavesArgon2>;

#[derive(Debug, Default)]
pub struct Estado {
    comandos: OnceLock<ComandosLimen>,
}

impl Estado {
    /// Se llama una sola vez, cuando la base terminó de abrir.
    pub fn listo(&self, app: AplicacionLimen) {
        drop(self.comandos.set(Comandos::new(app)));
    }

    /// Espera a que la base esté abierta.
    pub fn comandos(&self) -> &ComandosLimen {
        self.comandos.wait()
    }
}
