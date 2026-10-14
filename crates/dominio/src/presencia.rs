//! Presencia: quién está adentro y por qué vía (reglas E1 y A7).
//!
//! Una persona sólo puede estar adentro por una vía a la vez: si ya entró
//! como proveedor, no entra también como contratista, y no puede tener dos
//! ingresos abiertos por la misma vía. Se la reconoce por su
//! [`Identidad`]: la cédula normalizada o, para el personal KOF, su código
//! de empleado.
//!
//! El personal KOF está adentro mientras tenga un gafete provisional sin
//! devolver: entregarlo es su entrada y devolverlo, su salida.

use std::fmt;

use chrono::{DateTime, Utc};

use crate::cedula::Cedula;
use crate::origen::UnidadOperativaId;
use crate::personal_kof::CodigoEmpleado;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Via {
    Contratista,
    Proveedor,
    Correo,
    Kof,
}

impl Via {
    pub const TODAS: [Self; 4] = [Self::Contratista, Self::Proveedor, Self::Correo, Self::Kof];

    /// Código estable para guardar la vía.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Contratista => "CONTRATISTA",
            Self::Proveedor => "PROVEEDOR",
            Self::Correo => "CORREO",
            Self::Kof => "KOF",
        }
    }

    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|via| via.codigo() == codigo)
    }

    const fn descripcion(self) -> &'static str {
        match self {
            Self::Contratista => "contratista",
            Self::Proveedor => "proveedor",
            Self::Correo => "ingreso por correo",
            Self::Kof => "personal KOF",
        }
    }
}

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.descripcion())
    }
}

/// Cómo se reconoce a una persona para saber si está adentro: la cédula, o
/// el código de empleado si es del personal KOF (de quien no hay cédula).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identidad {
    Cedula(Cedula),
    Empleado(CodigoEmpleado),
}

impl Identidad {
    /// Texto estable que identifica a la persona en la base de datos. Las
    /// cédulas son sólo números y el código del KOF lleva un prefijo, así
    /// que dos personas distintas nunca comparten clave.
    pub fn clave(&self) -> String {
        match self {
            Self::Cedula(cedula) => cedula.as_str().to_owned(),
            Self::Empleado(codigo) => format!("KOF-{codigo}"),
        }
    }
}

/// Lo que se muestra: la cédula o el código de empleado, tal cual.
impl fmt::Display for Identidad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cedula(cedula) => f.write_str(cedula.as_str()),
            Self::Empleado(codigo) => f.write_str(codigo.as_str()),
        }
    }
}

impl From<&Cedula> for Identidad {
    fn from(cedula: &Cedula) -> Self {
        Self::Cedula(cedula.clone())
    }
}

impl From<&CodigoEmpleado> for Identidad {
    fn from(codigo: &CodigoEmpleado) -> Self {
        Self::Empleado(codigo.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Esta persona ya está adentro como {0}: registre primero su salida")]
pub struct YaEstaAdentro(pub Via);

impl YaEstaAdentro {
    pub const fn codigo(self) -> &'static str {
        "ya_esta_adentro"
    }
}

/// `Ok` si la persona no está adentro por ninguna vía.
pub const fn verificar_afuera(adentro_por: Option<Via>) -> Result<(), YaEstaAdentro> {
    match adentro_por {
        Some(via) => Err(YaEstaAdentro(via)),
        None => Ok(()),
    }
}

/// Regla A9: la nube dice que la persona ya está adentro en otra unidad.
/// El guarda no la registra aquí: pide la salida allá y después la ingresa.
/// Cómo se muestra la hora (zona del equipo) lo decide quien lo presenta;
/// el dominio sólo entrega el instante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdentroEnOtraUnidad {
    pub unidad: UnidadOperativaId,
    pub via: Via,
    pub desde: DateTime<Utc>,
}

impl AdentroEnOtraUnidad {
    pub const fn codigo(&self) -> &'static str {
        "adentro_en_otra_unidad"
    }
}

impl fmt::Display for AdentroEnOtraUnidad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Esta persona ya está adentro en {} como {}: pida allá su salida y después regístrela aquí",
            self.unidad.as_str(),
            self.via
        )
    }
}

impl std::error::Error for AdentroEnOtraUnidad {}

/// A9: `Ok` si la nube no reporta a la persona adentro en otra unidad.
/// Si la nube no respondió, quien llama pasa `None`: la portería no se
/// bloquea por la red y el choque, si lo hay, se marca al sincronizar.
/// Lo que reporta la propia unidad no cuenta: eso lo cubre E1 con la
/// presencia local.
pub fn verificar_no_adentro_en_otra_unidad(
    reportado: Option<AdentroEnOtraUnidad>,
    unidad_propia: &UnidadOperativaId,
) -> Result<(), AdentroEnOtraUnidad> {
    match reportado {
        Some(otra) if otra.unidad != *unidad_propia => Err(otra),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn otra(unidad: &str) -> AdentroEnOtraUnidad {
        AdentroEnOtraUnidad {
            unidad: UnidadOperativaId::nueva(unidad).unwrap(),
            via: Via::Contratista,
            desde: DateTime::<Utc>::from_timestamp(1_760_000_000, 0).unwrap(),
        }
    }

    #[test]
    fn a9_adentro_en_otra_unidad_bloquea_e_informa_donde() {
        let propia = UnidadOperativaId::nueva("BRISAS").unwrap();
        let error = verificar_no_adentro_en_otra_unidad(Some(otra("CARTAGO")), &propia)
            .expect_err("debe bloquear");
        assert_eq!(error.unidad.as_str(), "CARTAGO");
        assert_eq!(error.codigo(), "adentro_en_otra_unidad");
        assert_eq!(
            error.to_string(),
            "Esta persona ya está adentro en CARTAGO como contratista: pida allá su salida y después regístrela aquí"
        );
    }

    #[test]
    fn a9_sin_reporte_de_la_nube_la_porteria_no_se_bloquea() {
        let propia = UnidadOperativaId::nueva("BRISAS").unwrap();
        assert_eq!(verificar_no_adentro_en_otra_unidad(None, &propia), Ok(()));
    }

    #[test]
    fn a9_lo_de_la_propia_unidad_no_cuenta_porque_lo_cubre_e1() {
        let propia = UnidadOperativaId::nueva("BRISAS").unwrap();
        assert_eq!(
            verificar_no_adentro_en_otra_unidad(Some(otra("BRISAS")), &propia),
            Ok(())
        );
    }

    #[test]
    fn afuera_puede_entrar() {
        assert_eq!(verificar_afuera(None), Ok(()));
    }

    #[test]
    fn adentro_por_cualquier_via_bloquea_e_informa_cual() {
        for via in Via::TODAS {
            assert_eq!(verificar_afuera(Some(via)), Err(YaEstaAdentro(via)));
        }
        assert_eq!(
            YaEstaAdentro(Via::Proveedor).to_string(),
            "Esta persona ya está adentro como proveedor: registre primero su salida"
        );
    }

    #[test]
    fn los_codigos_van_y_vuelven() {
        for via in Via::TODAS {
            assert_eq!(Via::desde_codigo(via.codigo()), Some(via));
        }
    }

    #[test]
    fn la_clave_distingue_cedula_de_codigo_de_empleado() {
        let cedula = Cedula::normalizar("1-1111-1111").unwrap();
        let codigo = CodigoEmpleado::nuevo("5040017").unwrap();
        assert_eq!(Identidad::from(&cedula).clave(), "111111111");
        assert_eq!(Identidad::from(&codigo).clave(), "KOF-5040017");
        assert_ne!(
            Identidad::from(&cedula).clave(),
            Identidad::from(&codigo).clave()
        );
    }

    #[test]
    fn se_muestra_la_cedula_o_el_codigo_sin_prefijo() {
        let cedula = Cedula::normalizar("1-1111-1111").unwrap();
        let codigo = CodigoEmpleado::nuevo("5040017").unwrap();
        assert_eq!(Identidad::from(&cedula).to_string(), "111111111");
        assert_eq!(Identidad::from(&codigo).to_string(), "5040017");
    }

    #[test]
    fn el_personal_kof_es_una_via_mas() {
        assert_eq!(Via::Kof.codigo(), "KOF");
        assert_eq!(Via::desde_codigo("KOF"), Some(Via::Kof));
        assert_eq!(Via::Kof.to_string(), "personal KOF");
        assert_eq!(Via::TODAS.len(), 4);
    }
}
