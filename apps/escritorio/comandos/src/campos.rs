//! A qué campo de un formulario pertenece cada error, para que la interfaz
//! muestre el mensaje junto a él sin traducir códigos.
//!
//! Los nombres son las claves del JSON de entrada de cada formulario
//! ([`crate::ContratistaEntrada`], [`crate::EntradaProveedorEntrada`]…). Las
//! asignaciones van con `match` sin comodín: una regla nueva del dominio
//! obliga a decidir su campo. `None` es un error que no es de un campo (la
//! persona ya está adentro, el reloj atrasado): se muestra como mensaje
//! general.

use limen_aplicacion::errores::{ErrorCaso, ErrorDeNegocio};
use limen_dominio::contratista::ErrorContratista;
use limen_dominio::empresa::ErrorEmpresa;
use limen_dominio::gafete::{ErrorGafete, ErrorPrestamoGafete};
use limen_dominio::ingreso_contratista::ErrorIngreso;
use limen_dominio::ingreso_correo::{ErrorIngresoCorreo, ErrorMotivo};
use limen_dominio::ingreso_proveedor::ErrorIngresoProveedor;
use limen_dominio::medio::ErrorMedio;
use limen_dominio::personal_kof::ErrorPersonalKof;
use limen_dominio::prestamo_kof::ErrorPrestamoKof;
use limen_dominio::rango_fechas::ErrorRango;
use limen_dominio::usuario::ErrorUsuario;
use limen_dominio::visitante::ErrorVisitante;

use crate::error::ErrorJson;

pub const CEDULA: &str = "cedula";
pub const NOMBRE: &str = "nombre";
pub const EMPRESA: &str = "empresa_id";
pub const TIPO_INGRESO: &str = "tipo_ingreso";
pub const FECHA_VENCIMIENTO_PRAIND: &str = "fecha_vencimiento_praind";
pub const CODIGO_EMPLEADO: &str = "codigo_empleado";
pub const MOTIVO: &str = "motivo";
pub const MEDIO: &str = "medio";
pub const PLACA: &str = "placa";
pub const GAFETE: &str = "gafete";
pub const TIPO_GAFETE: &str = "tipo";
pub const HASTA: &str = "hasta";
pub const PORTADOR: &str = "portador";
pub const PERSONA: &str = "personal_id";
pub const CONTRATISTA: &str = "contratista_id";
pub const CLAVE: &str = "clave";
pub const CLAVE_ACTUAL: &str = "clave_actual";
pub const ACTIVO: &str = "activo";
pub const DESDE: &str = "desde";

/// El campo del formulario al que pertenece un error de negocio.
pub trait CampoDelError {
    fn campo(&self) -> Option<&'static str>;
}

/// El error de un caso de uso, con el campo de su regla si tiene uno.
pub fn con_campo<N: ErrorDeNegocio + CampoDelError>(error: ErrorCaso<N>) -> ErrorJson {
    let campo = match &error {
        ErrorCaso::Negocio(regla) => regla.campo(),
        ErrorCaso::NoEncontrado | ErrorCaso::Tecnico(_) => None,
    };
    ErrorJson::from(error).en_campo(campo)
}

impl CampoDelError for ErrorContratista {
    fn campo(&self) -> Option<&'static str> {
        Some(match self {
            Self::CedulaVacia
            | Self::CedulaInvalida
            | Self::CedulaRepetida
            | Self::CedulaNoEditableAdentro => CEDULA,
            Self::NombreVacio | Self::NombreInvalido => NOMBRE,
            Self::EmpresaNoExiste => EMPRESA,
            Self::PraindVencido => FECHA_VENCIMIENTO_PRAIND,
        })
    }
}

/// El formulario de una empresa (de contratistas o proveedora) sólo tiene
/// el nombre.
impl CampoDelError for ErrorEmpresa {
    fn campo(&self) -> Option<&'static str> {
        Some(NOMBRE)
    }
}

impl CampoDelError for ErrorPersonalKof {
    fn campo(&self) -> Option<&'static str> {
        Some(match self {
            Self::CodigoVacio | Self::CodigoInvalido | Self::CodigoRepetido => CODIGO_EMPLEADO,
            Self::NombreVacio | Self::NombreInvalido => NOMBRE,
        })
    }
}

impl CampoDelError for ErrorUsuario {
    fn campo(&self) -> Option<&'static str> {
        match self {
            Self::CedulaVacia | Self::CedulaInvalida | Self::CedulaRepetida => Some(CEDULA),
            Self::NombreVacio | Self::NombreInvalido => Some(NOMBRE),
            Self::ClaveCorta | Self::ClaveLarga | Self::ClaveIgualALaCedula => Some(CLAVE),
            Self::ClaveActualIncorrecta => Some(CLAVE_ACTUAL),
            Self::NoSeDesactivaASiMismo => Some(ACTIVO),
            Self::YaHayUsuarios => None,
        }
    }
}

/// En el historial, un rango invertido se muestra junto a la fecha final.
impl CampoDelError for ErrorRango {
    fn campo(&self) -> Option<&'static str> {
        Some(match self {
            Self::Invertido => HASTA,
        })
    }
}

impl CampoDelError for ErrorVisitante {
    fn campo(&self) -> Option<&'static str> {
        Some(match self {
            Self::CedulaVacia | Self::CedulaInvalida => CEDULA,
            Self::NombreVacio | Self::NombreInvalido => NOMBRE,
        })
    }
}

impl CampoDelError for ErrorMedio {
    fn campo(&self) -> Option<&'static str> {
        Some(match self {
            Self::PlacaRequerida | Self::PlacaInvalida => PLACA,
        })
    }
}

impl CampoDelError for ErrorPrestamoGafete {
    fn campo(&self) -> Option<&'static str> {
        Some(match self {
            Self::NoRegistrado | Self::NoDisponible(_) | Self::Prestado => GAFETE,
        })
    }
}

impl CampoDelError for ErrorMotivo {
    fn campo(&self) -> Option<&'static str> {
        Some(match self {
            Self::Vacio | Self::DemasiadoLargo | Self::CaracteresRaros => MOTIVO,
        })
    }
}

impl CampoDelError for ErrorIngreso {
    fn campo(&self) -> Option<&'static str> {
        match self {
            Self::Medio(error) => error.campo(),
            Self::Gafete(error) => error.campo(),
            Self::GafeteRequerido => Some(GAFETE),
            Self::YaEstaAdentro(_) | Self::AccesoDenegado(_) => None,
        }
    }
}

impl CampoDelError for ErrorIngresoProveedor {
    fn campo(&self) -> Option<&'static str> {
        match self {
            Self::Visitante(error) => error.campo(),
            Self::Medio(error) => error.campo(),
            Self::Gafete(error) => error.campo(),
            Self::EmpresaNoExiste => Some(EMPRESA),
            Self::YaEstaAdentro(_) | Self::AccesoDenegado(_) => None,
        }
    }
}

impl CampoDelError for ErrorIngresoCorreo {
    fn campo(&self) -> Option<&'static str> {
        match self {
            Self::Visitante(error) => error.campo(),
            Self::Motivo(error) => error.campo(),
            Self::Medio(error) => error.campo(),
            Self::Gafete(error) => error.campo(),
            Self::YaEstaAdentro(_) | Self::AccesoDenegado(_) => None,
        }
    }
}

impl CampoDelError for ErrorPrestamoKof {
    fn campo(&self) -> Option<&'static str> {
        match self {
            Self::Gafete(error) => error.campo(),
            Self::PersonalInactivo => Some(PERSONA),
            Self::YaTienePrestamo => None,
        }
    }
}

/// Los gafetes se crean por rango (`desde`, `hasta`) y se cambian de estado
/// uno por uno: lo que falla en un cambio de estado es del gafete entero,
/// salvo el último portador de uno perdido.
impl CampoDelError for ErrorGafete {
    fn campo(&self) -> Option<&'static str> {
        match self {
            // Al crear por rango, un `desde` en cero ya es rango inválido:
            // un número inválido sólo sale de un cambio de estado.
            Self::RangoInvalido => Some(HASTA),
            Self::PortadorDeOtroTipo | Self::CedulaDelPortadorInvalida => Some(PORTADOR),
            Self::NumeroInvalido
            | Self::Repetido
            | Self::NoDisponible
            | Self::NoEstaPerdido
            | Self::EnUso => None,
        }
    }
}
