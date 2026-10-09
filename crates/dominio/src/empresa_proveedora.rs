//! Empresa proveedora (regla H1): un catálogo aparte de las empresas de
//! contratistas. Tiene su propio ID, así una empresa proveedora no se puede
//! asignar por error a un contratista (y al revés).
//!
//! El nombre sigue las mismas reglas que el de una empresa de contratistas
//! (obligatorio, en MAYÚSCULAS, sin repetir dentro de su catálogo): por eso
//! reutiliza [`NombreEmpresa`] y [`ErrorEmpresa`].

use std::fmt;

use uuid::Uuid;

use crate::auditoria::{CambioCampo, CamposAuditables, cambios_de_alta, diferencias};
use crate::empresa::{ErrorEmpresa, HechosEmpresa, NombreEmpresa};

/// Identificador global de una empresa proveedora (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EmpresaProveedoraId(Uuid);

impl EmpresaProveedoraId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for EmpresaProveedoraId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpresaProveedora {
    id: EmpresaProveedoraId,
    nombre: NombreEmpresa,
}

impl EmpresaProveedora {
    pub fn registrar(
        id: EmpresaProveedoraId,
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
        let antes = self.campos_auditables();
        self.nombre = nombre;
        Ok(diferencias(antes, self.campos_auditables()))
    }

    /// Lo que la auditoría registra del alta: todos los campos.
    pub fn cambios_de_alta(&self) -> Vec<CambioCampo> {
        cambios_de_alta(self.campos_auditables())
    }

    fn campos_auditables(&self) -> CamposAuditables {
        vec![("nombre", self.nombre.to_string())]
    }

    /// Reconstruye una empresa proveedora ya guardada.
    pub const fn restaurar(id: EmpresaProveedoraId, nombre: NombreEmpresa) -> Self {
        Self { id, nombre }
    }

    pub const fn id(&self) -> EmpresaProveedoraId {
        self.id
    }

    pub const fn nombre(&self) -> &NombreEmpresa {
        &self.nombre
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> EmpresaProveedoraId {
        EmpresaProveedoraId::desde_uuid(Uuid::from_u128(1))
    }

    fn libre() -> HechosEmpresa {
        HechosEmpresa::default()
    }

    fn en_uso() -> HechosEmpresa {
        HechosEmpresa {
            nombre_en_uso: true,
        }
    }

    #[test]
    fn registra_con_el_nombre_normalizado() {
        let empresa = EmpresaProveedora::registrar(id(), " gas  zeta ", libre()).unwrap();
        assert_eq!(empresa.nombre().as_str(), "GAS ZETA");
        assert_eq!(empresa.id(), id());
    }

    #[test]
    fn nombre_obligatorio_y_sin_repetir() {
        assert_eq!(
            EmpresaProveedora::registrar(id(), "  ", libre()),
            Err(ErrorEmpresa::NombreVacio)
        );
        assert_eq!(
            EmpresaProveedora::registrar(id(), "GAS ZETA", en_uso()),
            Err(ErrorEmpresa::NombreRepetido)
        );
    }

    #[test]
    fn renombrar_devuelve_el_cambio() {
        let mut empresa = EmpresaProveedora::registrar(id(), "GAS ZETA", libre()).unwrap();
        let cambios = empresa.renombrar("gas zeta s.a.", libre()).unwrap();
        assert_eq!(cambios.len(), 1, "sólo cambió el nombre");
        assert_eq!(cambios[0].antes, "GAS ZETA");
        assert_eq!(cambios[0].despues, "GAS ZETA S.A.");
    }

    #[test]
    fn conservar_el_propio_nombre_no_es_repetirlo() {
        let mut empresa = EmpresaProveedora::registrar(id(), "GAS ZETA", libre()).unwrap();
        assert_eq!(
            empresa.renombrar("gas zeta", en_uso()),
            Ok(Vec::new()),
            "el nombre en uso es el suyo"
        );
        assert_eq!(
            empresa.renombrar("OTRA", en_uso()),
            Err(ErrorEmpresa::NombreRepetido)
        );
    }

    #[test]
    fn el_alta_audita_el_nombre() {
        let empresa = EmpresaProveedora::registrar(id(), "GAS ZETA", libre()).unwrap();
        let alta = empresa.cambios_de_alta();
        assert_eq!(alta.len(), 1, "un solo campo");
        assert_eq!(alta[0].campo, "nombre");
        assert_eq!(alta[0].despues, "GAS ZETA");
    }
}
