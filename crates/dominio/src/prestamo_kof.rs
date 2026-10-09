//! Gafete provisional para el personal KOF (reglas K1 a K3 de
//! `docs/reglas.md`).
//!
//! El personal KOF tiene su propio carnet permanente. Cuando no lo trae, se
//! le presta un gafete provisional del inventario KOF (tipo
//! `ProvisionalKof`) y lo devuelve al irse. **Es su entrada y su salida**:
//! mientras tiene el provisional está adentro (aparece en la lista de
//! quienes están adentro, con la vía [`Via::Kof`]) y entregarlo o devolverlo
//! es un movimiento que respeta el reloj del equipo (E5).
//!
//! Al entregar se revisa, en este orden:
//! 1. el reloj no retrocedió respecto al último movimiento (E5);
//! 2. la persona está activa (K2);
//! 3. no tiene ya un provisional sin devolver, es decir, no está adentro
//!    (K3, A7);
//! 4. el gafete existe en el inventario, está disponible y nadie más lo
//!    tiene (K1, K3).
//!
//! El préstamo guarda el código y el nombre de la persona tal como eran al
//! entregarlo, para mostrarlos sin buscar a la persona.

use std::fmt;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::gafete::{ErrorPrestamoGafete, NumeroGafete, SituacionGafete, verificar_prestamo};
use crate::movimiento::Marca;
use crate::nombre::NombrePersona;
use crate::personal_kof::{CodigoEmpleado, PersonalKof, PersonalKofId};
use crate::presencia::{Identidad, Via};
use crate::reloj::{RelojAtrasado, verificar_reloj};

/// Identificador global de un préstamo de gafete provisional (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PrestamoKofId(Uuid);

impl PrestamoKofId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for PrestamoKofId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Lo que el caso de uso averiguó antes de pedir la decisión.
#[derive(Debug, Clone, Copy)]
pub struct HechosEntregaKof {
    pub ultimo_movimiento: Option<DateTime<Utc>>,
    /// La persona ya tiene un provisional sin devolver.
    pub ya_tiene_prestamo: bool,
    /// Situación del gafete provisional indicado.
    pub situacion_gafete: SituacionGafete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorPrestamoKof {
    #[error("{0}")]
    Reloj(RelojAtrasado),
    #[error("La persona está inactiva: no se le puede prestar un gafete")]
    PersonalInactivo,
    #[error("Esta persona ya tiene un gafete provisional sin devolver")]
    YaTienePrestamo,
    #[error("{0}")]
    Gafete(ErrorPrestamoGafete),
}

impl ErrorPrestamoKof {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Reloj(error) => error.codigo(),
            Self::PersonalInactivo => "personal_kof_inactivo",
            Self::YaTienePrestamo => "personal_kof_con_prestamo",
            Self::Gafete(error) => error.codigo(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorDevolucionKof {
    #[error("Este gafete provisional ya fue devuelto")]
    YaDevuelto,
    #[error("{0}")]
    Reloj(RelojAtrasado),
    #[error("La devolución no puede ser anterior a la entrega")]
    AnteriorALaEntrega,
}

impl ErrorDevolucionKof {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::YaDevuelto => "prestamo_kof_ya_devuelto",
            Self::Reloj(error) => error.codigo(),
            Self::AnteriorALaEntrega => "devolucion_anterior_a_la_entrega",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrestamoKof {
    id: PrestamoKofId,
    personal: PersonalKofId,
    codigo: CodigoEmpleado,
    nombre: NombrePersona,
    gafete: NumeroGafete,
    entrega: Marca,
    devolucion: Option<Marca>,
}

/// Datos de un préstamo guardado, para reconstruirlo.
#[derive(Debug, Clone)]
pub struct PrestamoKofGuardado {
    pub id: PrestamoKofId,
    pub personal: PersonalKofId,
    pub codigo: CodigoEmpleado,
    pub nombre: NombrePersona,
    pub gafete: NumeroGafete,
    pub entrega: Marca,
    pub devolucion: Option<Marca>,
}

impl PrestamoKof {
    pub fn entregar(
        id: PrestamoKofId,
        personal: &PersonalKof,
        gafete: NumeroGafete,
        hechos: HechosEntregaKof,
        entrega: Marca,
    ) -> Result<Self, ErrorPrestamoKof> {
        verificar_reloj(entrega.en, hechos.ultimo_movimiento).map_err(ErrorPrestamoKof::Reloj)?;
        if !personal.activo() {
            return Err(ErrorPrestamoKof::PersonalInactivo);
        }
        if hechos.ya_tiene_prestamo {
            return Err(ErrorPrestamoKof::YaTienePrestamo);
        }
        verificar_prestamo(hechos.situacion_gafete).map_err(ErrorPrestamoKof::Gafete)?;
        Ok(Self {
            id,
            personal: personal.id(),
            codigo: personal.codigo().clone(),
            nombre: personal.nombre().clone(),
            gafete,
            entrega,
            devolucion: None,
        })
    }

    /// Registra la devolución. El gafete queda libre con ella.
    pub fn devolver(
        &mut self,
        devolucion: Marca,
        ultimo_movimiento: Option<DateTime<Utc>>,
    ) -> Result<(), ErrorDevolucionKof> {
        if self.devolucion.is_some() {
            return Err(ErrorDevolucionKof::YaDevuelto);
        }
        verificar_reloj(devolucion.en, ultimo_movimiento).map_err(ErrorDevolucionKof::Reloj)?;
        if devolucion.en < self.entrega.en {
            return Err(ErrorDevolucionKof::AnteriorALaEntrega);
        }
        self.devolucion = Some(devolucion);
        Ok(())
    }

    pub fn restaurar(guardado: PrestamoKofGuardado) -> Self {
        Self {
            id: guardado.id,
            personal: guardado.personal,
            codigo: guardado.codigo,
            nombre: guardado.nombre,
            gafete: guardado.gafete,
            entrega: guardado.entrega,
            devolucion: guardado.devolucion,
        }
    }

    pub const fn id(&self) -> PrestamoKofId {
        self.id
    }

    pub const fn personal(&self) -> PersonalKofId {
        self.personal
    }

    pub const fn codigo(&self) -> &CodigoEmpleado {
        &self.codigo
    }

    pub const fn nombre(&self) -> &NombrePersona {
        &self.nombre
    }

    /// Cómo se reconoce a la persona en la presencia.
    pub fn identidad(&self) -> Identidad {
        Identidad::from(&self.codigo)
    }

    /// La vía por la que está adentro mientras tenga el gafete.
    pub const fn via() -> Via {
        Via::Kof
    }

    pub const fn gafete(&self) -> NumeroGafete {
        self.gafete
    }

    pub const fn entrega(&self) -> Marca {
        self.entrega
    }

    pub const fn devolucion(&self) -> Option<Marca> {
        self.devolucion
    }

    pub const fn esta_abierto(&self) -> bool {
        self.devolucion.is_none()
    }

    /// Desde cuándo lo tiene (para mostrar en la lista de pendientes).
    pub const fn desde(&self) -> DateTime<Utc> {
        self.entrega.en
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gafete::EstadoGafete;
    use crate::operador::OperadorId;
    use crate::personal_kof::HechosPersonalKof;

    fn marca(texto: &str) -> Marca {
        Marca {
            en: texto.parse().unwrap(),
            operador: OperadorId::desde_uuid(Uuid::from_u128(9)),
        }
    }

    fn persona(activa: bool) -> PersonalKof {
        let mut persona = PersonalKof::registrar(
            PersonalKofId::desde_uuid(Uuid::from_u128(1)),
            "5040017",
            "ANA",
            HechosPersonalKof::default(),
        )
        .unwrap();
        persona.editar("ANA", activa).unwrap();
        persona
    }

    const LIBRE: SituacionGafete = SituacionGafete::Registrado {
        estado: EstadoGafete::Disponible,
        prestado: false,
    };

    fn hechos() -> HechosEntregaKof {
        HechosEntregaKof {
            ultimo_movimiento: None,
            ya_tiene_prestamo: false,
            situacion_gafete: LIBRE,
        }
    }

    fn entregar(
        persona: &PersonalKof,
        hechos: HechosEntregaKof,
    ) -> Result<PrestamoKof, ErrorPrestamoKof> {
        PrestamoKof::entregar(
            PrestamoKofId::desde_uuid(Uuid::from_u128(7)),
            persona,
            NumeroGafete::nuevo(3).unwrap(),
            hechos,
            marca("2026-10-09T08:00:00Z"),
        )
    }

    #[test]
    fn entrega_a_una_persona_activa() {
        let prestamo = entregar(&persona(true), hechos()).unwrap();
        assert_eq!(prestamo.personal(), persona(true).id());
        assert_eq!(prestamo.gafete(), NumeroGafete::nuevo(3).unwrap());
        assert!(prestamo.esta_abierto(), "recién entregado");
    }

    #[test]
    fn una_persona_inactiva_no_recibe_gafete() {
        assert_eq!(
            entregar(&persona(false), hechos()),
            Err(ErrorPrestamoKof::PersonalInactivo)
        );
    }

    #[test]
    fn una_persona_no_tiene_dos_provisionales() {
        let mut con_prestamo = hechos();
        con_prestamo.ya_tiene_prestamo = true;
        assert_eq!(
            entregar(&persona(true), con_prestamo),
            Err(ErrorPrestamoKof::YaTienePrestamo)
        );
    }

    #[test]
    fn el_gafete_debe_existir_estar_disponible_y_libre() {
        let casos = [
            (
                SituacionGafete::NoRegistrado,
                ErrorPrestamoGafete::NoRegistrado,
            ),
            (
                SituacionGafete::Registrado {
                    estado: EstadoGafete::DeBaja,
                    prestado: false,
                },
                ErrorPrestamoGafete::NoDisponible(EstadoGafete::DeBaja),
            ),
            (
                SituacionGafete::Registrado {
                    estado: EstadoGafete::Disponible,
                    prestado: true,
                },
                ErrorPrestamoGafete::Prestado,
            ),
        ];
        for (situacion, error) in casos {
            let mut con_gafete = hechos();
            con_gafete.situacion_gafete = situacion;
            assert_eq!(
                entregar(&persona(true), con_gafete),
                Err(ErrorPrestamoKof::Gafete(error)),
                "{situacion:?}"
            );
        }
    }

    #[test]
    fn inactiva_pesa_mas_que_el_prestamo_previo_y_este_mas_que_el_gafete() {
        let todo_mal = HechosEntregaKof {
            ultimo_movimiento: None,
            ya_tiene_prestamo: true,
            situacion_gafete: SituacionGafete::NoRegistrado,
        };
        assert_eq!(
            entregar(&persona(false), todo_mal),
            Err(ErrorPrestamoKof::PersonalInactivo)
        );
        assert_eq!(
            entregar(&persona(true), todo_mal),
            Err(ErrorPrestamoKof::YaTienePrestamo)
        );
    }

    #[test]
    fn la_devolucion_cierra_una_sola_vez_y_no_antes_de_la_entrega() {
        let mut prestamo = entregar(&persona(true), hechos()).unwrap();
        assert_eq!(
            prestamo.devolver(marca("2026-10-09T07:00:00Z"), None),
            Err(ErrorDevolucionKof::AnteriorALaEntrega)
        );
        assert!(prestamo.esta_abierto(), "la devolución rechazada no cierra");
        prestamo
            .devolver(marca("2026-10-09T17:00:00Z"), None)
            .unwrap();
        assert!(!prestamo.esta_abierto(), "devuelto");
        assert_eq!(
            prestamo.devolver(marca("2026-10-09T18:00:00Z"), None),
            Err(ErrorDevolucionKof::YaDevuelto)
        );
    }

    #[test]
    fn entregar_y_devolver_respetan_el_reloj_del_equipo() {
        let atrasado = Some("2026-10-09T09:00:00Z".parse().unwrap());
        let mut con_reloj_atrasado = hechos();
        con_reloj_atrasado.ultimo_movimiento = atrasado;
        assert_eq!(
            entregar(&persona(true), con_reloj_atrasado),
            Err(ErrorPrestamoKof::Reloj(RelojAtrasado)),
            "la entrega es de las 08:00 y el último movimiento fue a las 09:00"
        );
        let mut prestamo = entregar(&persona(true), hechos()).unwrap();
        assert_eq!(
            prestamo.devolver(
                marca("2026-10-09T17:00:00Z"),
                Some("2026-10-09T18:00:00Z".parse().unwrap())
            ),
            Err(ErrorDevolucionKof::Reloj(RelojAtrasado))
        );
        assert!(prestamo.esta_abierto(), "no se cerró");
    }

    #[test]
    fn el_reloj_se_revisa_antes_que_la_persona_inactiva() {
        let mut todo_mal = hechos();
        todo_mal.ultimo_movimiento = Some("2026-10-09T09:00:00Z".parse().unwrap());
        assert_eq!(
            entregar(&persona(false), todo_mal),
            Err(ErrorPrestamoKof::Reloj(RelojAtrasado))
        );
    }

    #[test]
    fn el_prestamo_guarda_quien_es_la_persona_y_por_que_via_esta_adentro() {
        let prestamo = entregar(&persona(true), hechos()).unwrap();
        assert_eq!(prestamo.codigo().as_str(), "5040017");
        assert_eq!(prestamo.nombre().as_str(), "ANA");
        assert_eq!(prestamo.identidad().clave(), "KOF-5040017");
        assert_eq!(PrestamoKof::via(), Via::Kof);
    }

    #[test]
    fn todo_error_tiene_codigo() {
        assert_eq!(
            ErrorPrestamoKof::YaTienePrestamo.codigo(),
            "personal_kof_con_prestamo"
        );
        assert_eq!(
            ErrorPrestamoKof::Gafete(ErrorPrestamoGafete::Prestado).codigo(),
            "gafete_prestado"
        );
        assert_eq!(
            ErrorDevolucionKof::YaDevuelto.codigo(),
            "prestamo_kof_ya_devuelto"
        );
        assert_eq!(
            ErrorPrestamoKof::Reloj(RelojAtrasado).codigo(),
            "reloj_atrasado"
        );
        assert_eq!(
            ErrorDevolucionKof::Reloj(RelojAtrasado).codigo(),
            "reloj_atrasado"
        );
    }
}
