//! Ingreso por correo (bloque I de `docs/reglas.md`): una visita
//! autorizada por correo (por ejemplo, una entrevista).
//!
//! Igual que un proveedor, la persona no está en un catálogo: su cédula y su
//! nombre llegan con cada ingreso (ver [`crate::visitante`]). En lugar de
//! empresa lleva un motivo ("a quién visita"), y el gafete es de visita y
//! obligatorio. Al entrar se revisa, en este orden:
//! 1. el medio: en vehículo la placa es obligatoria (E2);
//! 2. el reloj no retrocedió respecto al último movimiento (E5);
//! 3. la persona no está adentro por ninguna vía (I2, A7). Pesa más que el
//!    veto: si está adentro, lo que corresponde es registrar su salida;
//! 4. la cédula no está vetada (A8);
//! 5. el gafete de visita: registrado, disponible y libre (I2, E3).

use std::fmt;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::cedula::Cedula;
use crate::gafete::{ErrorPrestamoGafete, NumeroGafete, SituacionGafete, verificar_prestamo};
use crate::medio::{ErrorMedio, Medio, TipoMedio};
use crate::movimiento::{ErrorSalida, Marca, cerrar};
use crate::presencia::{Via, YaEstaAdentro, verificar_afuera};
use crate::reloj::{RelojAtrasado, verificar_reloj};
use crate::visitante::{ErrorVisitante, PersonaVetada, Visitante, verificar_veto};

/// Largo máximo del motivo de una visita.
pub const LARGO_MAXIMO_MOTIVO: usize = 200;

/// Identificador global de un ingreso por correo (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IngresoCorreoId(Uuid);

impl IngresoCorreoId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for IngresoCorreoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorMotivo {
    #[error("El motivo de la visita es obligatorio")]
    Vacio,
    #[error("El motivo admite hasta 200 caracteres")]
    DemasiadoLargo,
    #[error("El motivo tiene un carácter que no se puede guardar; bórrelo y escríbalo de nuevo")]
    CaracteresRaros,
}

impl ErrorMotivo {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Vacio => "motivo_vacio",
            Self::DemasiadoLargo => "motivo_largo",
            Self::CaracteresRaros => "motivo_caracteres_raros",
        }
    }
}

/// A quién visita o para qué viene: texto libre, sin espacios de más. Se
/// guarda como lo escribió el operador (no se pasa a mayúsculas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motivo(String);

impl Motivo {
    pub fn nuevo(texto: &str) -> Result<Self, ErrorMotivo> {
        let limpio = texto.split_whitespace().collect::<Vec<_>>().join(" ");
        if limpio.is_empty() {
            return Err(ErrorMotivo::Vacio);
        }
        if limpio.chars().count() > LARGO_MAXIMO_MOTIVO {
            return Err(ErrorMotivo::DemasiadoLargo);
        }
        if limpio.chars().any(char::is_control) {
            return Err(ErrorMotivo::CaracteresRaros);
        }
        Ok(Self(limpio))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Motivo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lo que llega del formulario, además de la persona y el motivo.
#[derive(Debug, Clone, Copy)]
pub struct DatosEntradaCorreo<'a> {
    pub medio: TipoMedio,
    pub placa: Option<&'a str>,
    /// Gafete de visita, siempre obligatorio.
    pub gafete: NumeroGafete,
}

/// Lo que el caso de uso averiguó antes de pedir la decisión.
#[derive(Debug, Clone, Copy)]
pub struct HechosEntradaCorreo {
    pub ultimo_movimiento: Option<DateTime<Utc>>,
    /// Por qué vía está adentro la persona (por su cédula), si lo está.
    pub adentro_por: Option<Via>,
    /// La cédula tiene el acceso denegado (A8).
    pub vetada: bool,
    /// Situación del gafete de visita indicado.
    pub situacion_gafete: SituacionGafete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorIngresoCorreo {
    #[error("{0}")]
    Visitante(ErrorVisitante),
    #[error("{0}")]
    Motivo(ErrorMotivo),
    #[error("{0}")]
    Medio(ErrorMedio),
    #[error("{0}")]
    Reloj(RelojAtrasado),
    #[error("{0}")]
    YaEstaAdentro(YaEstaAdentro),
    #[error("{0}")]
    AccesoDenegado(PersonaVetada),
    #[error("{0}")]
    Gafete(ErrorPrestamoGafete),
}

impl ErrorIngresoCorreo {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Visitante(error) => error.codigo(),
            Self::Motivo(error) => error.codigo(),
            Self::Medio(error) => error.codigo(),
            Self::Reloj(error) => error.codigo(),
            Self::YaEstaAdentro(error) => error.codigo(),
            Self::AccesoDenegado(error) => error.codigo(),
            Self::Gafete(error) => error.codigo(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoCorreo {
    id: IngresoCorreoId,
    visitante: Visitante,
    motivo: Motivo,
    medio: Medio,
    gafete: NumeroGafete,
    entrada: Marca,
    salida: Option<Marca>,
}

/// Datos de un ingreso por correo guardado, para reconstruirlo.
#[derive(Debug, Clone)]
pub struct IngresoCorreoGuardado {
    pub id: IngresoCorreoId,
    pub visitante: Visitante,
    pub motivo: Motivo,
    pub medio: Medio,
    pub gafete: NumeroGafete,
    pub entrada: Marca,
    pub salida: Option<Marca>,
}

impl IngresoCorreo {
    pub fn registrar_entrada(
        id: IngresoCorreoId,
        visitante: Visitante,
        motivo: Motivo,
        datos: DatosEntradaCorreo<'_>,
        hechos: HechosEntradaCorreo,
        entrada: Marca,
    ) -> Result<Self, ErrorIngresoCorreo> {
        let medio =
            Medio::desde_formulario(datos.medio, datos.placa).map_err(ErrorIngresoCorreo::Medio)?;
        verificar_reloj(entrada.en, hechos.ultimo_movimiento).map_err(ErrorIngresoCorreo::Reloj)?;
        verificar_afuera(hechos.adentro_por).map_err(ErrorIngresoCorreo::YaEstaAdentro)?;
        verificar_veto(hechos.vetada).map_err(ErrorIngresoCorreo::AccesoDenegado)?;
        verificar_prestamo(hechos.situacion_gafete).map_err(ErrorIngresoCorreo::Gafete)?;
        Ok(Self {
            id,
            visitante,
            motivo,
            medio,
            gafete: datos.gafete,
            entrada,
            salida: None,
        })
    }

    /// Registra la salida (E4, E5). El gafete queda libre con ella.
    pub fn registrar_salida(
        &mut self,
        salida: Marca,
        ultimo_movimiento: Option<DateTime<Utc>>,
    ) -> Result<(), ErrorSalida> {
        cerrar(self.entrada, &mut self.salida, salida, ultimo_movimiento)
    }

    pub fn restaurar(guardado: IngresoCorreoGuardado) -> Self {
        Self {
            id: guardado.id,
            visitante: guardado.visitante,
            motivo: guardado.motivo,
            medio: guardado.medio,
            gafete: guardado.gafete,
            entrada: guardado.entrada,
            salida: guardado.salida,
        }
    }

    pub const fn id(&self) -> IngresoCorreoId {
        self.id
    }

    pub const fn visitante(&self) -> &Visitante {
        &self.visitante
    }

    pub const fn cedula(&self) -> &Cedula {
        self.visitante.cedula()
    }

    pub const fn motivo(&self) -> &Motivo {
        &self.motivo
    }

    pub const fn medio(&self) -> &Medio {
        &self.medio
    }

    pub const fn gafete(&self) -> NumeroGafete {
        self.gafete
    }

    pub const fn entrada(&self) -> Marca {
        self.entrada
    }

    pub const fn salida(&self) -> Option<Marca> {
        self.salida
    }

    pub const fn esta_abierto(&self) -> bool {
        self.salida.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gafete::EstadoGafete;
    use crate::operador::OperadorId;

    const ENTRADA: &str = "2026-10-09T09:00:00Z";

    fn marca(texto: &str) -> Marca {
        Marca {
            en: texto.parse().unwrap(),
            operador: OperadorId::desde_uuid(Uuid::from_u128(9)),
        }
    }

    fn datos() -> DatosEntradaCorreo<'static> {
        DatosEntradaCorreo {
            medio: TipoMedio::APie,
            placa: Some("lo escrito se descarta"),
            gafete: NumeroGafete::nuevo(2).unwrap(),
        }
    }

    fn hechos() -> HechosEntradaCorreo {
        HechosEntradaCorreo {
            ultimo_movimiento: None,
            adentro_por: None,
            vetada: false,
            situacion_gafete: SituacionGafete::Registrado {
                estado: EstadoGafete::Disponible,
                prestado: false,
            },
        }
    }

    fn entrar(
        datos: DatosEntradaCorreo<'_>,
        hechos: HechosEntradaCorreo,
    ) -> Result<IngresoCorreo, ErrorIngresoCorreo> {
        IngresoCorreo::registrar_entrada(
            IngresoCorreoId::desde_uuid(Uuid::from_u128(1)),
            Visitante::nuevo("111111111", "ANA").unwrap(),
            Motivo::nuevo("Entrevista con Recursos Humanos").unwrap(),
            datos,
            hechos,
            marca(ENTRADA),
        )
    }

    #[test]
    fn el_motivo_se_limpia_y_conserva_lo_escrito() {
        let motivo = Motivo::nuevo("  Entrevista   con  RH ").unwrap();
        assert_eq!(motivo.as_str(), "Entrevista con RH");
    }

    #[test]
    fn el_motivo_es_obligatorio_y_acotado() {
        assert_eq!(Motivo::nuevo(" \t "), Err(ErrorMotivo::Vacio));
        let largo = "a".repeat(LARGO_MAXIMO_MOTIVO + 1);
        assert_eq!(Motivo::nuevo(&largo), Err(ErrorMotivo::DemasiadoLargo));
        let justo = "a".repeat(LARGO_MAXIMO_MOTIVO);
        assert!(Motivo::nuevo(&justo).is_ok(), "200 caracteres caben");
        assert_eq!(Motivo::nuevo("RH\u{0}"), Err(ErrorMotivo::CaracteresRaros));
    }

    #[test]
    fn entra_a_pie_con_gafete_de_visita() {
        let ingreso = entrar(datos(), hechos()).unwrap();
        assert_eq!(ingreso.medio(), &Medio::APie, "a pie descarta la placa");
        assert_eq!(ingreso.motivo().as_str(), "Entrevista con Recursos Humanos");
        assert_eq!(ingreso.gafete(), NumeroGafete::nuevo(2).unwrap());
        assert_eq!(ingreso.cedula().as_str(), "111111111");
        assert!(ingreso.esta_abierto(), "recién entra");
    }

    #[test]
    fn en_vehiculo_exige_placa() {
        let mut en_carro = datos();
        en_carro.medio = TipoMedio::Vehiculo;
        en_carro.placa = None;
        assert_eq!(
            entrar(en_carro, hechos()),
            Err(ErrorIngresoCorreo::Medio(ErrorMedio::PlacaRequerida))
        );
    }

    #[test]
    fn no_entra_con_el_reloj_atrasado() {
        let mut atrasado = hechos();
        atrasado.ultimo_movimiento = Some("2026-10-09T10:00:00Z".parse().unwrap());
        assert_eq!(
            entrar(datos(), atrasado),
            Err(ErrorIngresoCorreo::Reloj(RelojAtrasado))
        );
    }

    #[test]
    fn estar_adentro_pesa_mas_que_el_veto() {
        let mut ambos = hechos();
        ambos.adentro_por = Some(Via::Proveedor);
        ambos.vetada = true;
        assert_eq!(
            entrar(datos(), ambos),
            Err(ErrorIngresoCorreo::YaEstaAdentro(YaEstaAdentro(
                Via::Proveedor
            )))
        );
        let mut vetada = hechos();
        vetada.vetada = true;
        assert_eq!(
            entrar(datos(), vetada),
            Err(ErrorIngresoCorreo::AccesoDenegado(PersonaVetada))
        );
    }

    #[test]
    fn el_gafete_de_visita_debe_poder_prestarse() {
        let mut prestado = hechos();
        prestado.situacion_gafete = SituacionGafete::Registrado {
            estado: EstadoGafete::Disponible,
            prestado: true,
        };
        assert_eq!(
            entrar(datos(), prestado),
            Err(ErrorIngresoCorreo::Gafete(ErrorPrestamoGafete::Prestado))
        );
        let mut sin_registrar = hechos();
        sin_registrar.situacion_gafete = SituacionGafete::NoRegistrado;
        assert_eq!(
            entrar(datos(), sin_registrar),
            Err(ErrorIngresoCorreo::Gafete(
                ErrorPrestamoGafete::NoRegistrado
            ))
        );
    }

    #[test]
    fn la_salida_sigue_las_reglas_comunes() {
        let mut ingreso = entrar(datos(), hechos()).unwrap();
        ingreso
            .registrar_salida(marca("2026-10-09T10:00:00Z"), None)
            .unwrap();
        assert!(!ingreso.esta_abierto(), "ya salió");
        assert_eq!(
            ingreso.registrar_salida(marca("2026-10-09T11:00:00Z"), None),
            Err(ErrorSalida::YaSalio)
        );
    }

    #[test]
    fn todo_error_tiene_codigo() {
        assert_eq!(
            ErrorIngresoCorreo::Motivo(ErrorMotivo::Vacio).codigo(),
            "motivo_vacio"
        );
        assert_eq!(
            ErrorIngresoCorreo::AccesoDenegado(PersonaVetada).codigo(),
            "sin_acceso"
        );
    }
}
