//! Hechos: lo que ocurrió en el punto de acceso, tal como ocurrió.
//!
//! Cada entrada y cada salida (por las cuatro vías) deja un hecho que nunca
//! se edita ni se borra: si algo quedó mal, se corrige con otro hecho. Los
//! ingresos, la presencia y los préstamos de gafete son el estado que se
//! deriva de ellos; los hechos son el registro de lo que pasó y lo que se
//! enviará a los demás equipos al sincronizar (ver `docs/arquitectura.md`,
//! sección 9, y `docs/operacion-sin-conexion.md`).
//!
//! No confundir con los `Hechos*` de cada módulo (`HechosEntrada`…), que
//! son los datos que el caso de uso le entrega al dominio para decidir.
//!
//! Una entrada guarda el registro completo tal como se abrió, para que otro
//! equipo pueda reconstruirlo; una salida sólo dice qué registro se cerró,
//! cuándo y quién.
//!
//! Cada hecho guarda además si la hora con que se selló era confiable y la
//! hora cruda del reloj del equipo en ese momento (E5, ver
//! [`crate::reloj::sellar_hora`]): un hecho con hora dudosa no se pierde ni
//! detiene la portería, queda marcado para revisarlo.

use std::fmt;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::ingreso_contratista::IngresoContratista;
use crate::ingreso_correo::IngresoCorreo;
use crate::ingreso_proveedor::IngresoProveedor;
use crate::movimiento::Marca;
use crate::presencia::Via;
use crate::prestamo_kof::PrestamoKof;
use crate::reloj::HoraSellada;

/// Identificador de un hecho (UUID v7). Es ordenable: fija el orden en que
/// ocurrieron, aunque dos hechos compartan el mismo instante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HechoId(Uuid);

impl HechoId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for HechoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Qué ocurrió.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suceso {
    EntradaContratista(IngresoContratista),
    EntradaProveedor(IngresoProveedor),
    EntradaCorreo(IngresoCorreo),
    /// Entregar el gafete provisional es la entrada del personal KOF.
    EntregaKof(PrestamoKof),
    Salida(Salida),
}

/// Una salida: qué registro se cerró (el ingreso o, para el personal KOF,
/// el préstamo del provisional), cuándo y quién.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Salida {
    pub via: Via,
    pub registro: Uuid,
    pub marca: Marca,
}

/// Qué tan confiable fue la hora con que se selló un hecho (E5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalidadHora {
    /// `false`: el reloj retrocedió o la hora no se pudo comprobar contra
    /// una fuente externa. El hecho vale igual; queda para revisarlo.
    pub confiable: bool,
    /// La hora del reloj del equipo, tal cual, cuando se selló.
    pub hora_equipo: DateTime<Utc>,
}

impl From<HoraSellada> for CalidadHora {
    fn from(hora: HoraSellada) -> Self {
        Self {
            confiable: hora.confiable,
            hora_equipo: hora.hora_equipo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hecho {
    id: HechoId,
    suceso: Suceso,
    hora: CalidadHora,
}

impl Hecho {
    /// La entrada de un contratista, con el ingreso tal como se abrió.
    pub fn entrada_contratista(
        id: HechoId,
        ingreso: &IngresoContratista,
        hora: CalidadHora,
    ) -> Self {
        Self::restaurar(id, Suceso::EntradaContratista(ingreso.clone()), hora)
    }

    pub fn entrada_proveedor(id: HechoId, ingreso: &IngresoProveedor, hora: CalidadHora) -> Self {
        Self::restaurar(id, Suceso::EntradaProveedor(ingreso.clone()), hora)
    }

    pub fn entrada_correo(id: HechoId, ingreso: &IngresoCorreo, hora: CalidadHora) -> Self {
        Self::restaurar(id, Suceso::EntradaCorreo(ingreso.clone()), hora)
    }

    pub fn entrega_kof(id: HechoId, prestamo: &PrestamoKof, hora: CalidadHora) -> Self {
        Self::restaurar(id, Suceso::EntregaKof(prestamo.clone()), hora)
    }

    /// La salida de un contratista; `None` si el ingreso sigue abierto.
    pub fn salida_contratista(
        id: HechoId,
        ingreso: &IngresoContratista,
        hora: CalidadHora,
    ) -> Option<Self> {
        ingreso
            .salida()
            .map(|marca| Self::salida(id, Via::Contratista, ingreso.id().uuid(), marca, hora))
    }

    pub fn salida_proveedor(
        id: HechoId,
        ingreso: &IngresoProveedor,
        hora: CalidadHora,
    ) -> Option<Self> {
        ingreso
            .salida()
            .map(|marca| Self::salida(id, Via::Proveedor, ingreso.id().uuid(), marca, hora))
    }

    pub fn salida_correo(id: HechoId, ingreso: &IngresoCorreo, hora: CalidadHora) -> Option<Self> {
        ingreso
            .salida()
            .map(|marca| Self::salida(id, Via::Correo, ingreso.id().uuid(), marca, hora))
    }

    /// La devolución del provisional, que es la salida del personal KOF.
    pub fn devolucion_kof(id: HechoId, prestamo: &PrestamoKof, hora: CalidadHora) -> Option<Self> {
        prestamo
            .devolucion()
            .map(|marca| Self::salida(id, Via::Kof, prestamo.id().uuid(), marca, hora))
    }

    fn salida(id: HechoId, via: Via, registro: Uuid, marca: Marca, hora: CalidadHora) -> Self {
        Self::restaurar(
            id,
            Suceso::Salida(Salida {
                via,
                registro,
                marca,
            }),
            hora,
        )
    }

    /// Reconstruye un hecho leído de la base.
    pub const fn restaurar(id: HechoId, suceso: Suceso, hora: CalidadHora) -> Self {
        Self { id, suceso, hora }
    }

    /// Si la hora era confiable y la hora cruda del equipo.
    pub const fn hora(&self) -> CalidadHora {
        self.hora
    }

    pub const fn id(&self) -> HechoId {
        self.id
    }

    pub const fn suceso(&self) -> &Suceso {
        &self.suceso
    }

    pub const fn es_entrada(&self) -> bool {
        !matches!(self.suceso, Suceso::Salida(_))
    }

    /// Por qué vía entró o salió la persona.
    pub const fn via(&self) -> Via {
        match &self.suceso {
            Suceso::EntradaContratista(_) => Via::Contratista,
            Suceso::EntradaProveedor(_) => Via::Proveedor,
            Suceso::EntradaCorreo(_) => Via::Correo,
            Suceso::EntregaKof(_) => Via::Kof,
            Suceso::Salida(salida) => salida.via,
        }
    }

    /// El registro al que se refiere: el ingreso o el préstamo KOF.
    pub const fn registro(&self) -> Uuid {
        match &self.suceso {
            Suceso::EntradaContratista(ingreso) => ingreso.id().uuid(),
            Suceso::EntradaProveedor(ingreso) => ingreso.id().uuid(),
            Suceso::EntradaCorreo(ingreso) => ingreso.id().uuid(),
            Suceso::EntregaKof(prestamo) => prestamo.id().uuid(),
            Suceso::Salida(salida) => salida.registro,
        }
    }

    /// Cuándo ocurrió y quién lo registró.
    pub const fn marca(&self) -> Marca {
        match &self.suceso {
            Suceso::EntradaContratista(ingreso) => ingreso.entrada(),
            Suceso::EntradaProveedor(ingreso) => ingreso.entrada(),
            Suceso::EntradaCorreo(ingreso) => ingreso.entrada(),
            Suceso::EntregaKof(prestamo) => prestamo.entrega(),
            Suceso::Salida(salida) => salida.marca,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::cedula::Cedula;
    use crate::contratista::ContratistaId;
    use crate::ingreso_contratista::{EntregaGafete, IngresoGuardado, IngresoId};
    use crate::medio::Medio;
    use crate::operador::OperadorId;

    fn marca(texto: &str, operador: u128) -> Marca {
        Marca {
            en: texto.parse::<DateTime<Utc>>().unwrap(),
            operador: OperadorId::desde_uuid(Uuid::from_u128(operador)),
        }
    }

    fn ingreso(salida: Option<Marca>) -> IngresoContratista {
        IngresoContratista::restaurar(IngresoGuardado {
            id: IngresoId::desde_uuid(Uuid::from_u128(7)),
            contratista: ContratistaId::desde_uuid(Uuid::from_u128(8)),
            cedula: Cedula::normalizar("112345678").unwrap(),
            medio: Medio::APie,
            gafete: EntregaGafete::NoAplica,
            entrada: marca("2026-10-09T08:00:00Z", 1),
            salida,
        })
    }

    fn id(n: u128) -> HechoId {
        HechoId::desde_uuid(Uuid::from_u128(n))
    }

    fn hora() -> CalidadHora {
        CalidadHora {
            confiable: true,
            hora_equipo: "2026-10-09T08:00:02Z".parse().unwrap(),
        }
    }

    #[test]
    fn la_entrada_guarda_el_ingreso_tal_como_se_abrio() {
        let abierto = ingreso(None);
        let hecho = Hecho::entrada_contratista(id(1), &abierto, hora());
        assert_eq!(hecho.id(), id(1));
        assert_eq!(hecho.suceso(), &Suceso::EntradaContratista(abierto));
        assert!(hecho.es_entrada(), "una entrada es entrada");
        assert_eq!(hecho.via(), Via::Contratista);
        assert_eq!(hecho.registro(), Uuid::from_u128(7));
        assert_eq!(hecho.marca(), marca("2026-10-09T08:00:00Z", 1));
        assert_eq!(hecho.hora(), hora(), "con la calidad de su hora");
    }

    #[test]
    fn la_salida_dice_que_registro_se_cerro_cuando_y_quien() {
        let salida = marca("2026-10-09T17:00:00Z", 2);
        let hecho = Hecho::salida_contratista(id(2), &ingreso(Some(salida)), hora()).unwrap();
        assert!(!hecho.es_entrada(), "una salida no es entrada");
        assert_eq!(hecho.via(), Via::Contratista);
        assert_eq!(hecho.registro(), Uuid::from_u128(7));
        assert_eq!(hecho.marca(), salida);
    }

    #[test]
    fn un_ingreso_abierto_no_tiene_hecho_de_salida() {
        assert_eq!(
            Hecho::salida_contratista(id(3), &ingreso(None), hora()),
            None
        );
    }

    #[test]
    fn los_hechos_se_ordenan_por_su_id() {
        assert!(id(1) < id(2), "el UUID v7 es ordenable");
    }
}
