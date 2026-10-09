//! Empresa a la que pertenecen los contratistas.
//!
//! Regla de negocio: nombre obligatorio, en MAYÚSCULAS, sin espacios de más
//! y sin repetir. Una empresa no se bloquea ni se desactiva: si ya no tiene
//! trabajadores, simplemente nadie entra por ella.

use std::fmt;

use uuid::Uuid;

use crate::auditoria::{CambioCampo, anotar_si_cambia};

/// Largo máximo del nombre de una empresa.
pub const LARGO_MAXIMO_NOMBRE_EMPRESA: usize = 150;

/// Identificador global de una empresa (UUID v7: único entre equipos y
/// ordenado por fecha de creación). Lo genera quien crea la empresa, no el
/// dominio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EmpresaId(Uuid);

impl EmpresaId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for EmpresaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Nombre de empresa ya normalizado. A diferencia del nombre de persona,
/// admite números y signos ("ACME S.A.", "3M"), pero no caracteres de
/// control pegados desde otros programas.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NombreEmpresa(String);

impl NombreEmpresa {
    pub fn nuevo(texto: &str) -> Result<Self, ErrorEmpresa> {
        let limpio = texto
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_uppercase();
        if limpio.is_empty() {
            return Err(ErrorEmpresa::NombreVacio);
        }
        if limpio.chars().count() > LARGO_MAXIMO_NOMBRE_EMPRESA {
            return Err(ErrorEmpresa::NombreDemasiadoLargo);
        }
        if limpio.chars().any(char::is_control) {
            return Err(ErrorEmpresa::NombreConCaracteresRaros);
        }
        Ok(Self(limpio))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NombreEmpresa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorEmpresa {
    #[error("El nombre de la empresa es obligatorio")]
    NombreVacio,
    #[error("El nombre de la empresa admite hasta 150 caracteres")]
    NombreDemasiadoLargo,
    #[error("El nombre tiene un carácter que no se puede guardar; bórrelo y escríbalo de nuevo")]
    NombreConCaracteresRaros,
    #[error("Ya existe una empresa con ese nombre")]
    NombreRepetido,
}

impl ErrorEmpresa {
    /// Código estable para que las interfaces distingan el motivo sin
    /// comparar textos.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::NombreVacio => "empresa_nombre_vacio",
            Self::NombreDemasiadoLargo => "empresa_nombre_largo",
            Self::NombreConCaracteresRaros => "empresa_nombre_caracteres_raros",
            Self::NombreRepetido => "empresa_nombre_repetido",
        }
    }
}

/// Lo que el caso de uso averiguó en la base antes de pedirle al dominio
/// que decida. El dominio no consulta datos: los recibe.
#[derive(Debug, Clone, Copy, Default)]
pub struct HechosEmpresa {
    /// Otra empresa ya usa el nombre normalizado.
    pub nombre_en_uso: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empresa {
    id: EmpresaId,
    nombre: NombreEmpresa,
}

impl Empresa {
    pub fn registrar(
        id: EmpresaId,
        nombre: &str,
        hechos: HechosEmpresa,
    ) -> Result<Self, ErrorEmpresa> {
        let nombre = NombreEmpresa::nuevo(nombre)?;
        if hechos.nombre_en_uso {
            return Err(ErrorEmpresa::NombreRepetido);
        }
        Ok(Self { id, nombre })
    }

    /// Cambia el nombre y devuelve lo que cambió, para la auditoría.
    pub fn renombrar(
        &mut self,
        nombre: &str,
        hechos: HechosEmpresa,
    ) -> Result<Vec<CambioCampo>, ErrorEmpresa> {
        let nombre = NombreEmpresa::nuevo(nombre)?;
        if nombre != self.nombre && hechos.nombre_en_uso {
            return Err(ErrorEmpresa::NombreRepetido);
        }
        let mut cambios = Vec::new();
        anotar_si_cambia(&mut cambios, "nombre", &self.nombre, &nombre);
        self.nombre = nombre;
        Ok(cambios)
    }

    /// Reconstruye una empresa ya guardada. Los tipos de los parámetros
    /// garantizan que el nombre ya pasó por la normalización.
    pub const fn restaurar(id: EmpresaId, nombre: NombreEmpresa) -> Self {
        Self { id, nombre }
    }

    pub const fn id(&self) -> EmpresaId {
        self.id
    }

    pub const fn nombre(&self) -> &NombreEmpresa {
        &self.nombre
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> EmpresaId {
        EmpresaId::desde_uuid(Uuid::nil())
    }

    #[test]
    fn normaliza_el_nombre() {
        let empresa = Empresa::registrar(id(), "  acme  s.a. ", HechosEmpresa::default()).unwrap();
        assert_eq!(empresa.nombre().as_str(), "ACME S.A.");
    }

    #[test]
    fn rechaza_vacio_largo_y_caracteres_raros() {
        let registrar = |nombre: &str| Empresa::registrar(id(), nombre, HechosEmpresa::default());
        assert_eq!(registrar("  "), Err(ErrorEmpresa::NombreVacio));
        assert_eq!(
            registrar(&"A".repeat(151)),
            Err(ErrorEmpresa::NombreDemasiadoLargo)
        );
        assert_eq!(
            registrar("ACME\u{7}"),
            Err(ErrorEmpresa::NombreConCaracteresRaros)
        );
    }

    #[test]
    fn no_repite_nombre() {
        let hechos = HechosEmpresa {
            nombre_en_uso: true,
        };
        assert_eq!(
            Empresa::registrar(id(), "ACME", hechos),
            Err(ErrorEmpresa::NombreRepetido)
        );
    }

    #[test]
    fn renombrar_audita_el_cambio_y_tolera_su_propio_nombre() {
        let mut empresa = Empresa::registrar(id(), "ACME", HechosEmpresa::default()).unwrap();
        let sin_cambio = empresa.renombrar(
            "acme",
            HechosEmpresa {
                nombre_en_uso: true,
            },
        );
        assert_eq!(
            sin_cambio,
            Ok(vec![]),
            "su propio nombre no cuenta como repetido"
        );

        let cambios = empresa
            .renombrar("ACME CR", HechosEmpresa::default())
            .unwrap();
        assert_eq!(cambios.len(), 1);
        assert_eq!(cambios[0].antes, "ACME");
        assert_eq!(cambios[0].despues, "ACME CR");
    }
}
