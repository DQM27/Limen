//! Lo común a todos los ingresos (contratista, proveedor, correo): las
//! marcas de entrada y salida y las reglas de la salida.
//!
//! - E4: la salida no puede ser anterior a la entrada.
//! - E5: la hora de cada marca ya viene sellada por [`crate::reloj::sellar_hora`]:
//!   nunca es anterior al último movimiento, así que una salida nunca es
//!   anterior a su entrada salvo que se pase a mano una marca mal formada.
//! - Un ingreso cerrado no se vuelve a cerrar.

use chrono::{DateTime, Utc};

use crate::operador::OperadorId;

/// Cuándo y quién registró una entrada o una salida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marca {
    pub en: DateTime<Utc>,
    pub operador: OperadorId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorSalida {
    #[error("Este ingreso ya tiene la salida registrada")]
    YaSalio,
    #[error("La salida no puede ser anterior a la entrada")]
    AnteriorALaEntrada,
}

impl ErrorSalida {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::YaSalio => "ya_salio",
            Self::AnteriorALaEntrada => "salida_anterior_a_la_entrada",
        }
    }
}

/// Aplica las reglas de la salida y, si se cumplen, la registra.
pub fn cerrar(entrada: Marca, salida: &mut Option<Marca>, nueva: Marca) -> Result<(), ErrorSalida> {
    if salida.is_some() {
        return Err(ErrorSalida::YaSalio);
    }
    if nueva.en < entrada.en {
        return Err(ErrorSalida::AnteriorALaEntrada);
    }
    *salida = Some(nueva);
    Ok(())
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn marca(texto: &str) -> Marca {
        Marca {
            en: texto.parse().unwrap(),
            operador: OperadorId::desde_uuid(Uuid::from_u128(1)),
        }
    }

    const ENTRADA: &str = "2026-10-09T08:00:00Z";

    #[test]
    fn registra_la_salida() {
        let mut salida = None;
        let nueva = marca("2026-10-09T17:00:00Z");
        assert_eq!(cerrar(marca(ENTRADA), &mut salida, nueva), Ok(()));
        assert_eq!(salida, Some(nueva));
    }

    #[test]
    fn salir_en_el_mismo_instante_de_entrar_vale() {
        let mut salida = None;
        assert_eq!(
            cerrar(marca(ENTRADA), &mut salida, marca(ENTRADA)),
            Ok(()),
            "E4 dice 'no anterior', no 'posterior'"
        );
    }

    #[test]
    fn no_se_cierra_dos_veces() {
        let mut salida = Some(marca("2026-10-09T12:00:00Z"));
        assert_eq!(
            cerrar(marca(ENTRADA), &mut salida, marca("2026-10-09T17:00:00Z")),
            Err(ErrorSalida::YaSalio)
        );
        assert_eq!(salida, Some(marca("2026-10-09T12:00:00Z")), "no cambió");
    }

    #[test]
    fn la_salida_no_puede_ser_anterior_a_la_entrada() {
        let mut salida = None;
        assert_eq!(
            cerrar(marca(ENTRADA), &mut salida, marca("2026-10-09T07:59:59Z")),
            Err(ErrorSalida::AnteriorALaEntrada)
        );
        assert_eq!(salida, None, "no se registró");
    }
}
