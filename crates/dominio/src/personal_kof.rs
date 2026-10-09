//! Personal KOF: el personal interno de FEMSA (bloque K de `docs/reglas.md`).
//!
//! A diferencia de contratistas y proveedores, se identifica por el código
//! de empleado de su carnet (no por cédula, que no tenemos). Es un catálogo:
//! una persona dada de baja se desactiva, no se borra, para conservar su
//! historial.

use std::fmt;

use uuid::Uuid;

use crate::auditoria::{CambioCampo, CamposAuditables, cambios_de_alta, diferencias};
use crate::nombre::{NombreInvalido, NombrePersona};

/// Largo mínimo y máximo del código de empleado (lo que trae el carnet).
pub const LARGO_MINIMO_CODIGO: usize = 5;
pub const LARGO_MAXIMO_CODIGO: usize = 7;

/// Identificador global de una persona del personal KOF (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PersonalKofId(Uuid);

impl PersonalKofId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for PersonalKofId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Código de empleado: sólo dígitos, de 5 a 7.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CodigoEmpleado(String);

impl CodigoEmpleado {
    pub fn nuevo(texto: &str) -> Result<Self, ErrorPersonalKof> {
        let limpio = texto.trim();
        if limpio.is_empty() {
            return Err(ErrorPersonalKof::CodigoVacio);
        }
        let largo = limpio.chars().count();
        let solo_digitos = limpio.chars().all(|c| c.is_ascii_digit());
        if !solo_digitos || !(LARGO_MINIMO_CODIGO..=LARGO_MAXIMO_CODIGO).contains(&largo) {
            return Err(ErrorPersonalKof::CodigoInvalido);
        }
        Ok(Self(limpio.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CodigoEmpleado {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorPersonalKof {
    #[error("El código de empleado es obligatorio")]
    CodigoVacio,
    #[error("El código de empleado debe tener sólo números, entre 5 y 7 dígitos")]
    CodigoInvalido,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("El nombre sólo puede tener letras y espacios")]
    NombreInvalido,
    #[error("Ya existe una persona con ese código de empleado")]
    CodigoRepetido,
}

impl ErrorPersonalKof {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::CodigoVacio => "codigo_empleado_vacio",
            Self::CodigoInvalido => "codigo_empleado_invalido",
            Self::NombreVacio => "nombre_vacio",
            Self::NombreInvalido => "nombre_invalido",
            Self::CodigoRepetido => "codigo_empleado_repetido",
        }
    }
}

/// Lo que el caso de uso averiguó antes de pedir la decisión.
#[derive(Debug, Clone, Copy, Default)]
pub struct HechosPersonalKof {
    /// Otra persona ya usa ese código de empleado.
    pub codigo_en_uso: bool,
}

fn nombre_de(texto: &str) -> Result<NombrePersona, ErrorPersonalKof> {
    NombrePersona::nuevo(texto).map_err(|error| match error {
        NombreInvalido::Vacio => ErrorPersonalKof::NombreVacio,
        NombreInvalido::CaracteresNoPermitidos => ErrorPersonalKof::NombreInvalido,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalKof {
    id: PersonalKofId,
    codigo: CodigoEmpleado,
    nombre: NombrePersona,
    activo: bool,
}

impl PersonalKof {
    /// Da de alta a una persona. Nace activa.
    pub fn registrar(
        id: PersonalKofId,
        codigo: &str,
        nombre: &str,
        hechos: HechosPersonalKof,
    ) -> Result<Self, ErrorPersonalKof> {
        let codigo = CodigoEmpleado::nuevo(codigo)?;
        let nombre = nombre_de(nombre)?;
        if hechos.codigo_en_uso {
            return Err(ErrorPersonalKof::CodigoRepetido);
        }
        Ok(Self {
            id,
            codigo,
            nombre,
            activo: true,
        })
    }

    /// Cambia el nombre y si está activa. El código de empleado no se edita:
    /// es la identidad de la persona. Devuelve lo que cambió.
    pub fn editar(
        &mut self,
        nombre: &str,
        activo: bool,
    ) -> Result<Vec<CambioCampo>, ErrorPersonalKof> {
        let nombre = nombre_de(nombre)?;
        let antes = self.campos_auditables();
        self.nombre = nombre;
        self.activo = activo;
        Ok(diferencias(antes, self.campos_auditables()))
    }

    /// Lo que la auditoría registra del alta: todos los campos.
    pub fn cambios_de_alta(&self) -> Vec<CambioCampo> {
        cambios_de_alta(self.campos_auditables())
    }

    fn campos_auditables(&self) -> CamposAuditables {
        vec![
            ("codigo_empleado", self.codigo.to_string()),
            ("nombre", self.nombre.to_string()),
            ("activo", self.activo.to_string()),
        ]
    }

    /// Reconstruye una persona ya guardada.
    pub const fn restaurar(
        id: PersonalKofId,
        codigo: CodigoEmpleado,
        nombre: NombrePersona,
        activo: bool,
    ) -> Self {
        Self {
            id,
            codigo,
            nombre,
            activo,
        }
    }

    pub const fn id(&self) -> PersonalKofId {
        self.id
    }

    pub const fn codigo(&self) -> &CodigoEmpleado {
        &self.codigo
    }

    pub const fn nombre(&self) -> &NombrePersona {
        &self.nombre
    }

    pub const fn activo(&self) -> bool {
        self.activo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> PersonalKofId {
        PersonalKofId::desde_uuid(Uuid::from_u128(1))
    }

    fn libre() -> HechosPersonalKof {
        HechosPersonalKof::default()
    }

    #[test]
    fn el_codigo_son_de_5_a_7_digitos() {
        for valido in ["12345", "123456", "5040017", " 5040017 "] {
            assert!(CodigoEmpleado::nuevo(valido).is_ok(), "{valido:?}");
        }
        assert_eq!(
            CodigoEmpleado::nuevo("5040017").unwrap().as_str(),
            "5040017"
        );
        for invalido in ["1234", "12345678", "12A45", "12 345", "1-2345"] {
            assert_eq!(
                CodigoEmpleado::nuevo(invalido),
                Err(ErrorPersonalKof::CodigoInvalido),
                "{invalido:?}"
            );
        }
        assert_eq!(
            CodigoEmpleado::nuevo("  "),
            Err(ErrorPersonalKof::CodigoVacio)
        );
    }

    #[test]
    fn registra_activa_y_con_el_nombre_normalizado() {
        let persona = PersonalKof::registrar(id(), "5040017", "michael araya", libre()).unwrap();
        assert_eq!(persona.nombre().as_str(), "MICHAEL ARAYA");
        assert_eq!(persona.codigo().as_str(), "5040017");
        assert!(persona.activo(), "nace activa");
    }

    #[test]
    fn el_codigo_no_se_repite() {
        let en_uso = HechosPersonalKof {
            codigo_en_uso: true,
        };
        assert_eq!(
            PersonalKof::registrar(id(), "5040017", "ANA", en_uso),
            Err(ErrorPersonalKof::CodigoRepetido)
        );
    }

    #[test]
    fn valida_el_codigo_y_el_nombre_antes_del_hecho() {
        let en_uso = HechosPersonalKof {
            codigo_en_uso: true,
        };
        assert_eq!(
            PersonalKof::registrar(id(), "12", "ANA", en_uso),
            Err(ErrorPersonalKof::CodigoInvalido)
        );
        assert_eq!(
            PersonalKof::registrar(id(), "5040017", " ", libre()),
            Err(ErrorPersonalKof::NombreVacio)
        );
        assert_eq!(
            PersonalKof::registrar(id(), "5040017", "ANA 2", libre()),
            Err(ErrorPersonalKof::NombreInvalido)
        );
    }

    #[test]
    fn editar_devuelve_solo_lo_que_cambio() {
        let mut persona = PersonalKof::registrar(id(), "5040017", "ANA", libre()).unwrap();
        let cambios = persona.editar("ana", false).unwrap();
        assert_eq!(cambios.len(), 1, "el nombre sigue igual; cambió activo");
        assert_eq!(cambios[0].campo, "activo");
        assert_eq!(cambios[0].antes, "true");
        assert_eq!(cambios[0].despues, "false");
        assert!(!persona.activo());
        assert_eq!(persona.editar("ANA", false), Ok(Vec::new()), "sin cambios");
    }

    #[test]
    fn editar_rechaza_un_nombre_invalido_sin_tocar_nada() {
        let mut persona = PersonalKof::registrar(id(), "5040017", "ANA", libre()).unwrap();
        assert_eq!(
            persona.editar("ANA 2", false),
            Err(ErrorPersonalKof::NombreInvalido)
        );
        assert!(persona.activo(), "no cambió");
    }

    #[test]
    fn el_alta_audita_todos_los_campos() {
        let persona = PersonalKof::registrar(id(), "5040017", "ANA", libre()).unwrap();
        let campos: Vec<_> = persona.cambios_de_alta().iter().map(|c| c.campo).collect();
        assert_eq!(campos, ["codigo_empleado", "nombre", "activo"]);
    }
}
