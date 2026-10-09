//! Acceso al entrar: si un contratista puede pasar hoy.
//!
//! Reglas, en este orden (la primera que aplica decide):
//! 1. sin acceso → denegado ("acceso denegado" pesa más que el PRAIND);
//! 2. PRAIND vencido → denegado;
//! 3. PRAIND que vence en 30 días o menos → entra con advertencia;
//! 4. si no, entra.

use chrono::NaiveDate;

use crate::contratista::Contratista;
use crate::praind::{DIAS_ADVERTENCIA_PRAIND, dias_para_vencer, praind_vencido};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoAcceso {
    Permitido,
    /// Entra, pero se avisa cuántos días le quedan al PRAIND (0 = vence hoy).
    PermitidoConAdvertencia {
        dias_para_vencer: i64,
    },
    Denegado(MotivoDenegacion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MotivoDenegacion {
    #[error("Acceso denegado para esta persona")]
    SinAcceso,
    #[error("El PRAIND está vencido")]
    PraindVencido,
}

impl MotivoDenegacion {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::SinAcceso => "sin_acceso",
            Self::PraindVencido => "praind_vencido",
        }
    }
}

pub fn verificar_acceso(contratista: &Contratista, hoy: NaiveDate) -> ResultadoAcceso {
    if !contratista.tiene_acceso() {
        return ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso);
    }
    let vence = contratista.fecha_vencimiento_praind();
    if praind_vencido(vence, hoy) {
        return ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido);
    }
    let dias = dias_para_vencer(vence, hoy);
    if dias <= DIAS_ADVERTENCIA_PRAIND {
        return ResultadoAcceso::PermitidoConAdvertencia {
            dias_para_vencer: dias,
        };
    }
    ResultadoAcceso::Permitido
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::cedula::Cedula;
    use crate::contratista::{ContratistaGuardado, ContratistaId};
    use crate::empresa::EmpresaId;
    use crate::nombre::NombrePersona;
    use crate::tipo_ingreso::TipoIngreso;

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    fn contratista(vence: &str, tiene_acceso: bool) -> Contratista {
        Contratista::restaurar(ContratistaGuardado {
            id: ContratistaId::desde_uuid(Uuid::from_u128(1)),
            cedula: Cedula::normalizar("111111111").unwrap(),
            nombre: NombrePersona::nuevo("ANA").unwrap(),
            empresa: EmpresaId::desde_uuid(Uuid::from_u128(2)),
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: fecha(vence),
            tiene_acceso,
        })
    }

    const HOY: &str = "2026-10-09";

    #[test]
    fn sin_acceso_pesa_mas_que_el_praind_vencido() {
        assert_eq!(
            verificar_acceso(&contratista("2026-01-01", false), fecha(HOY)),
            ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso)
        );
    }

    #[test]
    fn praind_vencido_deniega() {
        assert_eq!(
            verificar_acceso(&contratista("2026-10-08", true), fecha(HOY)),
            ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
        );
    }

    #[test]
    fn advierte_desde_30_dias_hasta_el_mismo_dia() {
        assert_eq!(
            verificar_acceso(&contratista("2026-11-08", true), fecha(HOY)),
            ResultadoAcceso::PermitidoConAdvertencia {
                dias_para_vencer: 30
            }
        );
        assert_eq!(
            verificar_acceso(&contratista(HOY, true), fecha(HOY)),
            ResultadoAcceso::PermitidoConAdvertencia {
                dias_para_vencer: 0
            }
        );
    }

    #[test]
    fn con_mas_de_30_dias_entra_sin_aviso() {
        assert_eq!(
            verificar_acceso(&contratista("2026-11-09", true), fecha(HOY)),
            ResultadoAcceso::Permitido
        );
    }
}
