//! Vinculación de equipos con un código de un solo uso (bloque M).
//!
//! Reglas (como en Lattis, simplificadas a propósito):
//! - M6: el administrador **registra** un equipo desde el panel y recibe un código de
//!   [`LARGO_CODIGO`] caracteres. Vale [`VIGENCIA_CODIGO_MINUTOS`] minutos y sirve **una
//!   sola vez**. El alfabeto no trae los caracteres que se confunden (0/O, 1/I).
//! - M7: sólo hay dos acciones sobre un equipo: **registrar** y **retirar**. Un equipo
//!   reinstalado se registra como nuevo y el anterior se retira.
//! - M8: el código se guarda sólo como huella (SHA-256, que calcula la infraestructura);
//!   el dominio no lo guarda ni lo imprime en los errores.
//!
//! El azar y la hora los recibe de afuera, como todo el dominio.

use std::fmt;

use chrono::{DateTime, Duration, Utc};

/// Los caracteres que puede llevar un código: sin 0, 1, I ni O. Son 32, así que cada
/// byte al azar se reparte sin sesgo (`byte % 32`).
pub const ALFABETO: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
/// Cuántos caracteres tiene un código.
pub const LARGO_CODIGO: usize = 10;
/// Cuánto vale un código desde que se emite.
pub const VIGENCIA_CODIGO_MINUTOS: i64 = 15;

/// Un código de vinculación en su forma canónica: diez caracteres del [`ALFABETO`].
#[derive(Clone, PartialEq, Eq)]
pub struct CodigoVinculacion(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorVinculacion {
    #[error("El código debe tener {LARGO_CODIGO} letras y números, sin 0, 1, I ni O")]
    CodigoInvalido,
    #[error("El código ya se usó")]
    CodigoUsado,
    #[error("El código venció")]
    CodigoVencido,
    #[error("El código fue anulado")]
    CodigoAnulado,
    #[error("El equipo fue retirado")]
    EquipoRetirado,
    #[error("El equipo todavía no se ha vinculado")]
    EquipoSinVincular,
}

impl ErrorVinculacion {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::CodigoInvalido => "codigo_invalido",
            Self::CodigoUsado => "codigo_usado",
            Self::CodigoVencido => "codigo_vencido",
            Self::CodigoAnulado => "codigo_anulado",
            Self::EquipoRetirado => "equipo_retirado",
            Self::EquipoSinVincular => "equipo_sin_vincular",
        }
    }
}

impl CodigoVinculacion {
    /// Arma un código con el azar que le entregan (un byte por carácter).
    #[must_use]
    pub fn desde_azar(azar: [u8; LARGO_CODIGO]) -> Self {
        let letras = azar
            .iter()
            .map(|byte| {
                char::from(
                    ALFABETO
                        .get(usize::from(byte % 32))
                        .copied()
                        .unwrap_or(b'2'),
                )
            })
            .collect();
        Self(letras)
    }

    /// Lee lo que escribió o escaneó una persona: sin importar mayúsculas, espacios ni
    /// guiones (`ab3de-fgh45` es válido).
    pub fn leer(texto: &str) -> Result<Self, ErrorVinculacion> {
        let limpio: String = texto
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let valido =
            limpio.len() == LARGO_CODIGO && limpio.bytes().all(|byte| ALFABETO.contains(&byte));
        if valido {
            Ok(Self(limpio))
        } else {
            Err(ErrorVinculacion::CodigoInvalido)
        }
    }

    /// Los diez caracteres, sin separador (lo que se transforma en huella).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Como se le muestra a la persona: `XXXXX-XXXXX`.
    pub fn para_mostrar(&self) -> String {
        let (a, b) = self.0.split_at(LARGO_CODIGO / 2);
        format!("{a}-{b}")
    }
}

// El código nunca aparece en los registros técnicos.
impl fmt::Debug for CodigoVinculacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CodigoVinculacion(***)")
    }
}

/// ¿Puede este equipo conectarse a la nube? Sólo uno vinculado y no retirado (regla M2: un
/// equipo retirado pierde el acceso). Lo retirado se avisa antes que lo no vinculado.
pub const fn puede_conectarse(vinculado: bool, retirado: bool) -> Result<(), ErrorVinculacion> {
    if retirado {
        Err(ErrorVinculacion::EquipoRetirado)
    } else if !vinculado {
        Err(ErrorVinculacion::EquipoSinVincular)
    } else {
        Ok(())
    }
}

/// Lo que la nube sabe de un código ya emitido (la huella la compara la infraestructura).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmisionDeCodigo {
    pub emitido_en: DateTime<Utc>,
    pub usado: bool,
    pub anulado: bool,
}

impl EmisionDeCodigo {
    /// Cuándo deja de valer.
    pub fn vence_en(&self) -> DateTime<Utc> {
        self.emitido_en + Duration::minutes(VIGENCIA_CODIGO_MINUTOS)
    }

    /// ¿Se puede canjear ahora? El orden importa: lo anulado y lo usado se avisan antes
    /// que lo vencido, para que el evento de seguridad diga lo más grave.
    pub fn puede_canjearse(&self, ahora: DateTime<Utc>) -> Result<(), ErrorVinculacion> {
        if self.anulado {
            Err(ErrorVinculacion::CodigoAnulado)
        } else if self.usado {
            Err(ErrorVinculacion::CodigoUsado)
        } else if ahora >= self.vence_en() {
            Err(ErrorVinculacion::CodigoVencido)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn hoy() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 13, 12, 0, 0).unwrap()
    }

    fn emision(usado: bool, anulado: bool) -> EmisionDeCodigo {
        EmisionDeCodigo {
            emitido_en: hoy(),
            usado,
            anulado,
        }
    }

    #[test]
    fn el_azar_arma_un_codigo_valido_con_cualquier_byte() {
        for byte in 0..=u8::MAX {
            let codigo = CodigoVinculacion::desde_azar([byte; LARGO_CODIGO]);
            assert!(
                CodigoVinculacion::leer(codigo.as_str()).is_ok(),
                "el byte {byte} dio un código que no se puede leer"
            );
        }
    }

    #[test]
    fn el_alfabeto_no_trae_caracteres_que_se_confunden() {
        for confuso in b"01IO" {
            assert!(!ALFABETO.contains(confuso), "{}", char::from(*confuso));
        }
    }

    #[test]
    fn se_lee_sin_importar_mayusculas_espacios_ni_guiones() {
        let esperado = CodigoVinculacion::leer("AB3DEFGH45").unwrap();
        assert_eq!(CodigoVinculacion::leer("ab3de-fgh45").unwrap(), esperado);
        assert_eq!(CodigoVinculacion::leer(" AB3DE FGH45 ").unwrap(), esperado);
    }

    #[test]
    fn rechaza_largo_incorrecto_y_caracteres_fuera_del_alfabeto() {
        for malo in [
            "",
            "AB3DEFGH4",
            "AB3DEFGH455",
            "AB3DEFGH40",
            "AB3DEFGHI5",
            "AB3DEFGH4!",
        ] {
            assert_eq!(
                CodigoVinculacion::leer(malo),
                Err(ErrorVinculacion::CodigoInvalido),
                "{malo:?}"
            );
        }
    }

    #[test]
    fn se_muestra_en_dos_grupos_y_no_se_imprime_en_los_registros() {
        let codigo = CodigoVinculacion::leer("AB3DEFGH45").unwrap();
        assert_eq!(codigo.para_mostrar(), "AB3DE-FGH45");
        assert!(
            !format!("{codigo:?}").contains("AB3DE"),
            "el código no debe salir en Debug"
        );
    }

    #[test]
    fn un_codigo_nuevo_se_puede_canjear_hasta_que_vence() {
        let nuevo = emision(false, false);
        assert_eq!(nuevo.puede_canjearse(hoy()), Ok(()));
        let justo_antes = hoy() + Duration::minutes(VIGENCIA_CODIGO_MINUTOS) - Duration::seconds(1);
        assert_eq!(nuevo.puede_canjearse(justo_antes), Ok(()));
        let al_vencer = hoy() + Duration::minutes(VIGENCIA_CODIGO_MINUTOS);
        assert_eq!(
            nuevo.puede_canjearse(al_vencer),
            Err(ErrorVinculacion::CodigoVencido)
        );
    }

    #[test]
    fn un_codigo_usado_o_anulado_no_se_canjea_aunque_este_vigente() {
        assert_eq!(
            emision(true, false).puede_canjearse(hoy()),
            Err(ErrorVinculacion::CodigoUsado)
        );
        assert_eq!(
            emision(false, true).puede_canjearse(hoy()),
            Err(ErrorVinculacion::CodigoAnulado)
        );
    }

    #[test]
    fn anulado_y_usado_a_la_vez_avisa_lo_anulado() {
        assert_eq!(
            emision(true, true).puede_canjearse(hoy() + Duration::hours(1)),
            Err(ErrorVinculacion::CodigoAnulado)
        );
    }

    #[test]
    fn solo_se_conecta_un_equipo_vinculado_y_no_retirado() {
        assert_eq!(puede_conectarse(true, false), Ok(()));
        assert_eq!(
            puede_conectarse(false, false),
            Err(ErrorVinculacion::EquipoSinVincular)
        );
        assert_eq!(
            puede_conectarse(true, true),
            Err(ErrorVinculacion::EquipoRetirado)
        );
        assert_eq!(
            puede_conectarse(false, true),
            Err(ErrorVinculacion::EquipoRetirado),
            "retirado se avisa antes que sin vincular"
        );
    }
}
