//! Lo que Tauri comparte entre todos los comandos.

use limen_composicion::AplicacionLimen;
use limen_escritorio_comandos::{Comandos, OperadorDelEquipo};
use limen_infra_plataforma::{IdsV7, RelojCostaRica};
use limen_infra_surreal::AlmacenSurreal;

/// Los comandos con los adaptadores reales.
pub type ComandosLimen = Comandos<AlmacenSurreal, RelojCostaRica, IdsV7>;

/// Estado administrado por Tauri, sin candado global: la concurrencia de la
/// base la maneja su adaptador.
#[derive(Debug)]
pub struct Estado {
    comandos: ComandosLimen,
    /// TEMPORAL hasta el bloque L (ver `limen_escritorio_comandos::operador`).
    operador: OperadorDelEquipo,
}

impl Estado {
    pub const fn new(app: AplicacionLimen, operador: OperadorDelEquipo) -> Self {
        Self {
            comandos: Comandos::new(app),
            operador,
        }
    }

    pub const fn comandos(&self) -> &ComandosLimen {
        &self.comandos
    }

    pub const fn operador(&self) -> &OperadorDelEquipo {
        &self.operador
    }
}
