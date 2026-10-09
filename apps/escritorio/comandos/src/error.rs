//! El error que ve la interfaz: siempre la misma forma, venga de donde venga.
//!
//! Tres orígenes, un solo destino ([`ErrorJson`]):
//!
//! - una **regla de negocio** del dominio (cédula repetida, PRAIND vencido):
//!   conserva su código estable y su mensaje;
//! - un **dato de entrada ilegible** (un identificador mal formado, una
//!   fecha que no es fecha): lo detecta este crate antes de llamar al caso
//!   de uso, y es [`ErrorEntrada`];
//! - una **falla técnica**: al operador sólo le llega el mensaje genérico;
//!   el detalle va al registro (`log`), nunca a la pantalla.

use limen_aplicacion::errores::{ErrorCaso, ErrorDeNegocio, TipoError};
use serde::Serialize;

/// Cómo clasifica la interfaz un error: lo que el operador puede corregir
/// (`negocio`) o lo que debe atender quien mantiene el sistema (`tecnico`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoErrorJson {
    Negocio,
    Tecnico,
}

/// Lo que recibe la interfaz cuando un comando falla.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorJson {
    pub tipo: TipoErrorJson,
    /// Código estable: la interfaz decide por él, nunca comparando textos.
    pub codigo: &'static str,
    /// Mensaje para mostrar tal cual al operador.
    pub mensaje: String,
}

/// Un dato que llegó de la interfaz y no se pudo leer. Son errores del
/// operador o de la propia interfaz, no de una regla: por eso viven aquí y
/// no en el dominio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorEntrada {
    #[error("El identificador no es válido")]
    IdInvalido,
    #[error("La fecha no es válida: use año-mes-día")]
    FechaInvalida,
    #[error("El tipo de ingreso no es válido")]
    TipoIngresoInvalido,
    #[error("El medio de ingreso no es válido")]
    MedioInvalido,
    #[error("La vía de ingreso no es válida")]
    ViaInvalida,
}

impl ErrorEntrada {
    /// Código estable, igual que los de las reglas del dominio.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::IdInvalido => "id_invalido",
            Self::FechaInvalida => "fecha_invalida",
            Self::TipoIngresoInvalido => "tipo_ingreso_invalido",
            Self::MedioInvalido => "medio_invalido",
            Self::ViaInvalida => "via_invalida",
        }
    }
}

impl From<ErrorEntrada> for ErrorJson {
    fn from(error: ErrorEntrada) -> Self {
        Self {
            tipo: TipoErrorJson::Negocio,
            codigo: error.codigo(),
            mensaje: error.to_string(),
        }
    }
}

impl<N: ErrorDeNegocio> From<ErrorCaso<N>> for ErrorJson {
    fn from(error: ErrorCaso<N>) -> Self {
        let interfaz = error.para_interfaz();
        if let Some(detalle) = &interfaz.detalle_tecnico {
            log::error!("falla técnica ({}): {detalle}", interfaz.codigo);
        }
        Self {
            tipo: match interfaz.tipo {
                TipoError::Negocio => TipoErrorJson::Negocio,
                TipoError::Tecnico => TipoErrorJson::Tecnico,
            },
            codigo: interfaz.codigo,
            mensaje: interfaz.mensaje,
        }
    }
}
