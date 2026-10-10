//! Cómo viaja un error desde la base hasta la pantalla.
//!
//! Hay dos tipos y se tratan distinto:
//!
//! - **De negocio**: los resuelve el operador corrigiendo el dato (cédula
//!   repetida, PRAIND vencido). Se le muestra el mensaje exacto de la regla.
//! - **Técnico**: lo resuelve quien mantiene el sistema (base dañada). Al
//!   operador se le muestra un mensaje genérico y el detalle va al registro.
//!
//! Cada caso de uso devuelve [`ErrorCaso`], y la interfaz lo convierte con
//! [`ErrorCaso::para_interfaz`]: un solo traductor para todas las pantallas.

use std::fmt;

use limen_dominio::contratista::ErrorContratista;
use limen_dominio::empresa::ErrorEmpresa;
use limen_dominio::gafete::ErrorGafete;
use limen_dominio::ingreso_contratista::ErrorIngreso;
use limen_dominio::ingreso_correo::ErrorIngresoCorreo;
use limen_dominio::ingreso_proveedor::ErrorIngresoProveedor;
use limen_dominio::movimiento::ErrorSalida;
use limen_dominio::personal_kof::ErrorPersonalKof;
use limen_dominio::prestamo_kof::{ErrorDevolucionKof, ErrorPrestamoKof};
use limen_dominio::rango_fechas::ErrorRango;
use limen_dominio::usuario::{ErrorInicioSesion, ErrorUsuario};

use crate::puertos::{ErrorPersistencia, Restriccion};

/// Mensaje que ve el operador ante una falla técnica.
pub const MENSAJE_ERROR_TECNICO: &str =
    "No se pudo completar la operación. Intente de nuevo; si sigue fallando, avise a soporte.";

/// Error de negocio del dominio, con su código estable.
pub trait ErrorDeNegocio: fmt::Display {
    fn codigo(&self) -> &'static str;
}

impl ErrorDeNegocio for ErrorContratista {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorEmpresa {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorIngreso {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorSalida {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorGafete {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorRango {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

/// Las consultas no tienen error de negocio: este tipo no tiene valores, así
/// que el código nunca llega a usarse. Existe para que `ErrorCaso` se pueda
/// traducir a la interfaz igual que el de cualquier otro caso de uso.
impl ErrorDeNegocio for std::convert::Infallible {
    fn codigo(&self) -> &'static str {
        "sin_regla"
    }
}

impl ErrorDeNegocio for ErrorPersonalKof {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorPrestamoKof {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorDevolucionKof {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorIngresoCorreo {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorIngresoProveedor {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorUsuario {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

impl ErrorDeNegocio for ErrorInicioSesion {
    fn codigo(&self) -> &'static str {
        Self::codigo(*self)
    }
}

/// Lo que puede salir mal en un caso de uso cuyo error de negocio es `N`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorCaso<N> {
    #[error("{0}")]
    Negocio(N),
    #[error("El registro no existe")]
    NoEncontrado,
    #[error("falla técnica: {0}")]
    Tecnico(String),
}

impl<N> From<ErrorPersistencia> for ErrorCaso<N> {
    /// Una lectura no puede chocar con una restricción: si llega un
    /// conflicto acá, es una falla técnica. Al confirmar se usa
    /// [`ErrorCaso::al_confirmar`], que sí traduce los conflictos.
    fn from(error: ErrorPersistencia) -> Self {
        Self::Tecnico(error.to_string())
    }
}

impl<N> ErrorCaso<N> {
    /// Traduce el error de `confirmar()`: un conflicto con una restricción
    /// de la base se convierte en el error de negocio equivalente (el mismo
    /// que da la regla del dominio), para que el operador vea lo mismo venga
    /// de donde venga.
    pub fn al_confirmar(
        error: ErrorPersistencia,
        traducir: impl FnOnce(Restriccion) -> Option<N>,
    ) -> Self {
        match error {
            ErrorPersistencia::Conflicto(restriccion) => traducir(restriccion).map_or_else(
                || Self::Tecnico(ErrorPersistencia::Conflicto(restriccion).to_string()),
                Self::Negocio,
            ),
            ErrorPersistencia::Tecnica(detalle) => Self::Tecnico(detalle),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoError {
    Negocio,
    Tecnico,
}

/// Lo que recibe la interfaz: siempre la misma forma, para cualquier caso
/// de uso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorParaInterfaz {
    pub tipo: TipoError,
    pub codigo: &'static str,
    pub mensaje: String,
    /// Sólo en errores técnicos: el detalle para el registro, nunca para la
    /// pantalla.
    pub detalle_tecnico: Option<String>,
}

impl<N: ErrorDeNegocio> ErrorCaso<N> {
    pub fn para_interfaz(&self) -> ErrorParaInterfaz {
        match self {
            Self::Negocio(error) => ErrorParaInterfaz {
                tipo: TipoError::Negocio,
                codigo: error.codigo(),
                mensaje: error.to_string(),
                detalle_tecnico: None,
            },
            Self::NoEncontrado => ErrorParaInterfaz {
                tipo: TipoError::Negocio,
                codigo: "no_encontrado",
                mensaje: Self::NoEncontrado.to_string(),
                detalle_tecnico: None,
            },
            Self::Tecnico(detalle) => ErrorParaInterfaz {
                tipo: TipoError::Tecnico,
                codigo: "error_tecnico",
                mensaje: MENSAJE_ERROR_TECNICO.to_owned(),
                detalle_tecnico: Some(detalle.clone()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Error = ErrorCaso<ErrorContratista>;

    #[test]
    fn el_conflicto_de_cedula_se_traduce_al_error_de_negocio() {
        let error = Error::al_confirmar(
            ErrorPersistencia::Conflicto(Restriccion::CedulaContratista),
            |restriccion| {
                (restriccion == Restriccion::CedulaContratista)
                    .then_some(ErrorContratista::CedulaRepetida)
            },
        );
        assert_eq!(error, Error::Negocio(ErrorContratista::CedulaRepetida));
    }

    #[test]
    fn un_conflicto_sin_traduccion_es_tecnico() {
        let error = Error::al_confirmar(
            ErrorPersistencia::Conflicto(Restriccion::NombreEmpresa),
            |_| None,
        );
        assert!(matches!(error, Error::Tecnico(_)), "{error:?}");
    }

    #[test]
    fn un_conflicto_en_una_lectura_es_tecnico() {
        let error: Error = ErrorPersistencia::Conflicto(Restriccion::CedulaContratista).into();
        assert!(matches!(error, Error::Tecnico(_)), "{error:?}");
    }

    #[test]
    fn el_negocio_muestra_el_mensaje_de_la_regla() {
        let interfaz = Error::Negocio(ErrorContratista::PraindVencido).para_interfaz();
        assert_eq!(interfaz.tipo, TipoError::Negocio);
        assert_eq!(interfaz.codigo, "praind_vencido");
        assert_eq!(
            interfaz.mensaje,
            "El PRAIND está vencido: ingrese una fecha vigente"
        );
        assert_eq!(interfaz.detalle_tecnico, None);
    }

    #[test]
    fn lo_tecnico_no_llega_a_la_pantalla() {
        let interfaz = Error::Tecnico("disco lleno".into()).para_interfaz();
        assert_eq!(interfaz.tipo, TipoError::Tecnico);
        assert_eq!(interfaz.mensaje, MENSAJE_ERROR_TECNICO);
        assert!(
            !interfaz.mensaje.contains("disco"),
            "el detalle no se muestra"
        );
        assert_eq!(interfaz.detalle_tecnico.as_deref(), Some("disco lleno"));
    }

    #[test]
    fn no_encontrado_tiene_codigo_propio() {
        let interfaz = Error::NoEncontrado.para_interfaz();
        assert_eq!(interfaz.codigo, "no_encontrado");
        assert_eq!(interfaz.tipo, TipoError::Negocio);
    }
}
