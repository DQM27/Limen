//! OPERADOR PROVISIONAL — TEMPORAL.
//!
//! Los usuarios se van a crear en el panel de la nube (bloque L de
//! `docs/reglas.md`), que todavía no existe. Mientras tanto, la primera vez
//! que se abre la app en un equipo se pide un nombre y se guarda un operador
//! en un archivo local. Todo esto se reemplaza cuando exista el inicio de
//! sesión real (L2 a L5); no debe crecer.
//!
//! El nombre es sólo para mostrar: la auditoría registra el identificador.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use limen_aplicacion::errores::ErrorCaso;
use limen_aplicacion::puertos::GeneradorIds;
use limen_aplicacion::sesion::{OperadorId, Sesion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ErrorJson, TipoErrorJson};

/// El operador de este equipo, tal como se guarda y se muestra.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operador {
    pub id: Uuid,
    pub nombre: String,
}

impl Operador {
    pub const fn sesion(&self) -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(self.id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorOperador {
    #[error("Escriba el nombre del operador")]
    NombreVacio,
    #[error("No se pudo leer o guardar el operador: {0}")]
    Archivo(String),
}

impl From<ErrorOperador> for ErrorJson {
    fn from(error: ErrorOperador) -> Self {
        match error {
            ErrorOperador::NombreVacio => Self {
                tipo: TipoErrorJson::Negocio,
                codigo: "nombre_vacio",
                mensaje: error.to_string(),
            },
            ErrorOperador::Archivo(detalle) => {
                ErrorCaso::<std::convert::Infallible>::Tecnico(detalle).into()
            }
        }
    }
}

/// El operador de este equipo y el archivo donde se guarda.
#[derive(Debug)]
pub struct OperadorDelEquipo {
    ruta: PathBuf,
    actual: Mutex<Option<Operador>>,
}

impl OperadorDelEquipo {
    /// Lee el operador guardado en `ruta`, si ya se creó.
    pub fn abrir(ruta: PathBuf) -> Result<Self, ErrorOperador> {
        let guardado = match fs::read_to_string(&ruta) {
            Ok(texto) => Some(
                serde_json::from_str(&texto)
                    .map_err(|error| ErrorOperador::Archivo(error.to_string()))?,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(ErrorOperador::Archivo(error.to_string())),
        };
        Ok(Self {
            ruta,
            actual: Mutex::new(guardado),
        })
    }

    /// Un `Mutex` envenenado por un panic en otro comando no debe dejar la
    /// app inutilizable: el dato sigue siendo válido.
    fn guardado(&self) -> MutexGuard<'_, Option<Operador>> {
        self.actual.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn actual(&self) -> Option<Operador> {
        self.guardado().clone()
    }

    /// La sesión con la que se registra todo. Sin operador no se registra
    /// nada, ni siquiera las lecturas: ningún comando se puede invocar
    /// directo desde la interfaz sin sesión.
    pub fn sesion(&self) -> Result<Sesion, ErrorJson> {
        self.actual()
            .map(|operador| operador.sesion())
            .ok_or_else(ErrorJson::sin_operador)
    }

    /// Crea y guarda el operador de este equipo. El identificador lo genera
    /// el puerto `ids` (este crate no genera IDs).
    pub fn crear(&self, ids: &impl GeneradorIds, nombre: &str) -> Result<Operador, ErrorOperador> {
        let nombre = nombre.split_whitespace().collect::<Vec<_>>().join(" ");
        if nombre.is_empty() {
            return Err(ErrorOperador::NombreVacio);
        }
        let operador = Operador {
            id: ids.nuevo(),
            nombre,
        };
        let texto = serde_json::to_string_pretty(&operador)
            .map_err(|error| ErrorOperador::Archivo(error.to_string()))?;
        fs::write(&self.ruta, texto).map_err(|error| ErrorOperador::Archivo(error.to_string()))?;
        *self.guardado() = Some(operador.clone());
        Ok(operador)
    }
}
