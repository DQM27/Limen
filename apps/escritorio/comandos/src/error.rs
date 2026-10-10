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
    /// El campo del formulario que causó el error (la clave del JSON de
    /// entrada, por ejemplo `cedula`), para mostrarlo junto a él. `None`
    /// cuando el error no es de un campo (un fallo técnico, un registro que
    /// no existe).
    pub campo: Option<&'static str>,
}

impl ErrorJson {
    /// Ningún operador ha iniciado sesión en este equipo: nada se registra
    /// sin saber quién lo hace (regla E6).
    pub fn sin_operador() -> Self {
        Self {
            tipo: TipoErrorJson::Negocio,
            codigo: "sin_operador",
            mensaje: "Falta identificar al operador de este equipo".to_owned(),
            campo: None,
        }
    }

    /// El mismo error, atribuido a un campo del formulario.
    #[must_use]
    pub const fn en_campo(mut self, campo: Option<&'static str>) -> Self {
        self.campo = campo;
        self
    }
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
    #[error("El tipo de gafete no es válido")]
    TipoGafeteInvalido,
    #[error("El cambio de estado del gafete no es válido")]
    CambioGafeteInvalido,
    #[error(
        "Indique un solo último portador del gafete: el contratista, la cédula de la persona o el personal KOF"
    )]
    PortadorInvalido,
    #[error("Indique el número de gafete o «Sin gafete», no los dos")]
    GafeteYSinGafete,
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
            Self::TipoGafeteInvalido => "tipo_gafete_invalido",
            Self::CambioGafeteInvalido => "cambio_gafete_invalido",
            Self::PortadorInvalido => "portador_invalido",
            Self::GafeteYSinGafete => "gafete_y_sin_gafete",
        }
    }
}

impl From<ErrorEntrada> for ErrorJson {
    fn from(error: ErrorEntrada) -> Self {
        Self {
            tipo: TipoErrorJson::Negocio,
            codigo: error.codigo(),
            mensaje: error.to_string(),
            campo: None,
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
            campo: None,
        }
    }
}
