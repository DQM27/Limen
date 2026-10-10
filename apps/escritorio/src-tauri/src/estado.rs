//! Lo que Tauri comparte entre todos los comandos.

use limen_composicion::AplicacionLimen;
use limen_escritorio_comandos::Comandos;
use limen_infra_plataforma::{ContrasenasArgon2, IdsV7, RelojCostaRica};
use limen_infra_surreal::AlmacenSurreal;

/// Los comandos con los adaptadores reales.
pub type ComandosLimen = Comandos<AlmacenSurreal, RelojCostaRica, IdsV7, ContrasenasArgon2>;

/// Estado administrado por Tauri, sin candado global: la concurrencia de la
/// base la maneja su adaptador, y la sesión del equipo vive en los comandos.
#[derive(Debug)]
pub struct Estado {
    comandos: ComandosLimen,
}

impl Estado {
    pub fn new(app: AplicacionLimen) -> Self {
        Self {
            comandos: Comandos::new(app),
        }
    }

    pub const fn comandos(&self) -> &ComandosLimen {
        &self.comandos
    }
}
