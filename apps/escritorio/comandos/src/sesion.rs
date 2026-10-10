//! La sesión abierta en este equipo (bloque L).
//!
//! Vive sólo en memoria: al cerrar la app se cierra la sesión y hay que
//! volver a entrar. Sin sesión ningún comando registra ni lee nada (regla
//! E6: todo queda a nombre de quien lo hizo); con una clave temporal
//! sólo se puede cambiarla o salir (lo decide el dominio,
//! `usuario::puede_operar`).

use std::sync::{Mutex, MutexGuard, PoisonError};

use limen_aplicacion::casos_de_uso::usuarios::SesionIniciada;
use limen_aplicacion::errores::ErrorCaso;
use limen_aplicacion::sesion::Sesion;
use limen_dominio::usuario::puede_operar;
use serde::Serialize;

use crate::error::ErrorJson;

/// Quién tiene la sesión, tal como lo ve la interfaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsuarioActual {
    pub id: String,
    pub cedula: String,
    pub nombre: String,
    /// La clave es temporal: la interfaz pide cambiarla antes de
    /// seguir (y los comandos no dejan hacer otra cosa).
    pub debe_cambiar_clave: bool,
}

impl From<&SesionIniciada> for UsuarioActual {
    fn from(iniciada: &SesionIniciada) -> Self {
        Self {
            id: iniciada.sesion.operador().uuid().to_string(),
            cedula: iniciada.cedula.clone(),
            nombre: iniciada.nombre.clone(),
            debe_cambiar_clave: iniciada.debe_cambiar_clave,
        }
    }
}

/// La sesión de este equipo: una a la vez.
#[derive(Debug, Default)]
pub struct SesionDelEquipo {
    actual: Mutex<Option<Abierta>>,
}

#[derive(Debug, Clone)]
struct Abierta {
    sesion: Sesion,
    usuario: UsuarioActual,
}

impl SesionDelEquipo {
    /// Un `Mutex` envenenado por un pánico en otro comando no debe dejar la
    /// app inutilizable: el dato sigue siendo válido.
    fn guardada(&self) -> MutexGuard<'_, Option<Abierta>> {
        self.actual.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn abierta(&self) -> Result<Abierta, ErrorJson> {
        self.guardada().clone().ok_or_else(ErrorJson::sin_sesion)
    }

    pub fn actual(&self) -> Option<UsuarioActual> {
        self.guardada()
            .as_ref()
            .map(|abierta| abierta.usuario.clone())
    }

    /// La sesión con la que se registra todo. Sin sesión, o con una
    /// clave temporal sin cambiar, no se hace nada.
    pub fn sesion(&self) -> Result<Sesion, ErrorJson> {
        let abierta = self.abierta()?;
        puede_operar(abierta.usuario.debe_cambiar_clave).map_err(ErrorCaso::Negocio)?;
        Ok(abierta.sesion)
    }

    /// La sesión aunque la clave sea temporal: sólo para cambiarla.
    pub(crate) fn sesion_para_cambiar_clave(&self) -> Result<Sesion, ErrorJson> {
        self.abierta().map(|abierta| abierta.sesion)
    }

    pub(crate) fn abrir(&self, iniciada: &SesionIniciada) -> UsuarioActual {
        let usuario = UsuarioActual::from(iniciada);
        *self.guardada() = Some(Abierta {
            sesion: iniciada.sesion.clone(),
            usuario: usuario.clone(),
        });
        usuario
    }

    /// La clave ya no es temporal.
    pub(crate) fn clave_cambiada(&self) {
        if let Some(abierta) = self.guardada().as_mut() {
            abierta.usuario.debe_cambiar_clave = false;
        }
    }

    pub fn cerrar(&self) {
        *self.guardada() = None;
    }
}
