//! Gafetes: el inventario físico de credenciales que se prestan al entrar.
//!
//! Reglas de negocio (bloque F de `docs/reglas.md`):
//! - F1: número mayor a cero, sin repetir dentro de su tipo; se pueden crear
//!   por rango.
//! - F2: tipos contratista, visita (la usa el ingreso por correo), proveedor
//!   y provisional KOF. Cada vía de ingreso usa sólo su tipo.
//! - F3: estados Disponible → Perdido → Disponible (pagado o apareció), y
//!   Disponible → De baja.
//! - F4: sólo un gafete disponible se da de baja o se marca perdido.
//! - F5: marcar perdido exige indicar su último portador (quién lo tenía), que
//!   corresponde al tipo del gafete; un proveedor o una visita, con su cédula
//!   nacional o de extranjero (A3).
//! - F6: no se da de baja un gafete prestado en este momento.
//!
//! Un gafete se identifica por su tipo y su número: el 25 de contratista y
//! el 25 de proveedor son gafetes distintos.

use std::fmt;

use crate::auditoria::{CambioCampo, CamposAuditables, cambios_de_alta, diferencias};
use crate::cedula::Cedula;
use crate::contratista::ContratistaId;
use crate::personal_kof::PersonalKofId;

/// Cuántos gafetes se pueden crear de una vez con un rango.
pub const MAXIMO_POR_RANGO: u32 = 1000;

/// Número impreso en el gafete. Siempre mayor a cero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NumeroGafete(u32);

impl NumeroGafete {
    pub const fn nuevo(numero: u32) -> Result<Self, ErrorGafete> {
        if numero == 0 {
            return Err(ErrorGafete::NumeroInvalido);
        }
        Ok(Self(numero))
    }

    pub const fn valor(self) -> u32 {
        self.0
    }
}

impl fmt::Display for NumeroGafete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TipoGafete {
    Contratista,
    /// Lo usa el ingreso por correo.
    Visita,
    Proveedor,
    ProvisionalKof,
}

impl TipoGafete {
    pub const TODOS: [Self; 4] = [
        Self::Contratista,
        Self::Visita,
        Self::Proveedor,
        Self::ProvisionalKof,
    ];

    /// Código estable para guardar e intercambiar el tipo.
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Contratista => "CONTRATISTA",
            Self::Visita => "VISITA",
            Self::Proveedor => "PROVEEDOR",
            Self::ProvisionalKof => "PROVISIONAL_KOF",
        }
    }

    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|tipo| tipo.codigo() == codigo)
    }
}

impl fmt::Display for TipoGafete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.codigo())
    }
}

/// El último portador de un gafete perdido: quién lo tenía (regla F5).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Portador {
    Contratista(ContratistaId),
    /// Proveedor o persona que entró por correo: no hay catálogo de esas
    /// personas, se identifican por su cédula.
    Persona(Cedula),
    PersonalKof(PersonalKofId),
}

impl Portador {
    /// Si puede haber tenido un gafete de ese tipo: el de contratista, un
    /// contratista; el de visita o de proveedor, una persona; el
    /// provisional, alguien del personal KOF.
    pub const fn corresponde_a(&self, tipo: TipoGafete) -> bool {
        matches!(
            (self, tipo),
            (Self::Contratista(_), TipoGafete::Contratista)
                | (Self::Persona(_), TipoGafete::Visita | TipoGafete::Proveedor)
                | (Self::PersonalKof(_), TipoGafete::ProvisionalKof)
        )
    }
}

impl fmt::Display for Portador {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contratista(id) => write!(f, "contratista {id}"),
            Self::Persona(cedula) => write!(f, "persona {cedula}"),
            Self::PersonalKof(id) => write!(f, "personal KOF {id}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EstadoGafete {
    Disponible,
    Perdido,
    DeBaja,
}

impl EstadoGafete {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Disponible => "DISPONIBLE",
            Self::Perdido => "PERDIDO",
            Self::DeBaja => "DE_BAJA",
        }
    }

    pub fn desde_codigo(codigo: &str) -> Option<Self> {
        [Self::Disponible, Self::Perdido, Self::DeBaja]
            .into_iter()
            .find(|estado| estado.codigo() == codigo)
    }
}

impl fmt::Display for EstadoGafete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.codigo())
    }
}

/// Cómo se resolvió un gafete perdido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolucion {
    Pagado,
    Aparecido,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorGafete {
    #[error("El número de gafete debe ser mayor a cero")]
    NumeroInvalido,
    #[error("El rango de números no es válido (desde ≤ hasta, hasta {MAXIMO_POR_RANGO} gafetes)")]
    RangoInvalido,
    #[error("Ya existe un gafete con ese número")]
    Repetido,
    #[error("El gafete no está disponible")]
    NoDisponible,
    #[error("El gafete no está perdido")]
    NoEstaPerdido,
    #[error("El gafete está prestado en este momento")]
    EnUso,
    #[error("El último portador no corresponde al tipo de gafete")]
    PortadorDeOtroTipo,
    #[error("La cédula del último portador debe ser nacional o de extranjero (9 a 13 dígitos)")]
    CedulaDelPortadorInvalida,
}

impl ErrorGafete {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::NumeroInvalido => "gafete_numero_invalido",
            Self::RangoInvalido => "gafete_rango_invalido",
            Self::Repetido => "gafete_repetido",
            Self::NoDisponible => "gafete_no_disponible",
            Self::NoEstaPerdido => "gafete_no_perdido",
            Self::EnUso => "gafete_en_uso",
            Self::PortadorDeOtroTipo => "gafete_portador_de_otro_tipo",
            Self::CedulaDelPortadorInvalida => "gafete_cedula_portador_invalida",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gafete {
    tipo: TipoGafete,
    numero: NumeroGafete,
    estado: EstadoGafete,
    portador: Option<Portador>,
}

impl Gafete {
    /// Gafete nuevo, disponible. `ya_existe` lo averigua el caso de uso.
    pub fn registrar(
        tipo: TipoGafete,
        numero: NumeroGafete,
        ya_existe: bool,
    ) -> Result<Self, ErrorGafete> {
        if ya_existe {
            return Err(ErrorGafete::Repetido);
        }
        Ok(Self {
            tipo,
            numero,
            estado: EstadoGafete::Disponible,
            portador: None,
        })
    }

    /// Los números de un rango (F1). Cuáles ya existen lo decide el caso de
    /// uso con [`Gafete::registrar`].
    pub fn rango(desde: u32, hasta: u32) -> Result<Vec<NumeroGafete>, ErrorGafete> {
        let cantidad = hasta.checked_sub(desde).map(|diferencia| diferencia + 1);
        match cantidad {
            Some(cantidad) if desde > 0 && cantidad <= MAXIMO_POR_RANGO => {
                (desde..=hasta).map(NumeroGafete::nuevo).collect()
            }
            _ => Err(ErrorGafete::RangoInvalido),
        }
    }

    /// F4 y F5: sólo un gafete disponible se marca perdido, y hay que decir
    /// quién lo tenía: alguien que corresponda a su tipo y, si es un
    /// proveedor o una visita, con cédula nacional o de extranjero (A3).
    pub fn marcar_perdido(&mut self, portador: Portador) -> Result<Vec<CambioCampo>, ErrorGafete> {
        if self.estado != EstadoGafete::Disponible {
            return Err(ErrorGafete::NoDisponible);
        }
        if !portador.corresponde_a(self.tipo) {
            return Err(ErrorGafete::PortadorDeOtroTipo);
        }
        if let Portador::Persona(cedula) = &portador
            && !cedula.es_nacional_o_de_extranjero()
        {
            return Err(ErrorGafete::CedulaDelPortadorInvalida);
        }
        Ok(self.cambiar(EstadoGafete::Perdido, Some(portador)))
    }

    /// F3: un gafete perdido vuelve a estar disponible cuando se paga o
    /// aparece. El motivo queda en la auditoría.
    pub fn resolver(&mut self, resolucion: Resolucion) -> Result<Vec<CambioCampo>, ErrorGafete> {
        if self.estado != EstadoGafete::Perdido {
            return Err(ErrorGafete::NoEstaPerdido);
        }
        let mut cambios = self.cambiar(EstadoGafete::Disponible, None);
        cambios.push(CambioCampo {
            campo: "resolucion",
            antes: String::new(),
            despues: match resolucion {
                Resolucion::Pagado => "PAGADO",
                Resolucion::Aparecido => "APARECIDO",
            }
            .to_owned(),
        });
        Ok(cambios)
    }

    /// F4 y F6: sólo un gafete disponible y que no está prestado ahora.
    pub fn dar_de_baja(&mut self, en_uso: bool) -> Result<Vec<CambioCampo>, ErrorGafete> {
        if self.estado != EstadoGafete::Disponible {
            return Err(ErrorGafete::NoDisponible);
        }
        if en_uso {
            return Err(ErrorGafete::EnUso);
        }
        Ok(self.cambiar(EstadoGafete::DeBaja, None))
    }

    /// Reconstruye un gafete guardado.
    pub const fn restaurar(
        tipo: TipoGafete,
        numero: NumeroGafete,
        estado: EstadoGafete,
        portador: Option<Portador>,
    ) -> Self {
        Self {
            tipo,
            numero,
            estado,
            portador,
        }
    }

    pub const fn tipo(&self) -> TipoGafete {
        self.tipo
    }

    pub const fn numero(&self) -> NumeroGafete {
        self.numero
    }

    pub const fn estado(&self) -> EstadoGafete {
        self.estado
    }

    pub const fn portador(&self) -> Option<&Portador> {
        self.portador.as_ref()
    }

    pub fn cambios_de_alta(&self) -> Vec<CambioCampo> {
        cambios_de_alta(self.campos_auditables())
    }

    fn cambiar(&mut self, estado: EstadoGafete, portador: Option<Portador>) -> Vec<CambioCampo> {
        let antes = self.campos_auditables();
        self.estado = estado;
        self.portador = portador;
        diferencias(antes, self.campos_auditables())
    }

    fn campos_auditables(&self) -> CamposAuditables {
        vec![
            ("tipo", self.tipo.to_string()),
            ("numero", self.numero.to_string()),
            ("estado", self.estado.to_string()),
            (
                "portador",
                self.portador
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            ),
        ]
    }
}

/// Lo que se sabe de un gafete al querer prestarlo (lo averigua el caso de
/// uso).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SituacionGafete {
    NoRegistrado,
    Registrado {
        estado: EstadoGafete,
        prestado: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorPrestamoGafete {
    #[error("El gafete no está registrado en el catálogo")]
    NoRegistrado,
    #[error("El gafete no está disponible (está {0})")]
    NoDisponible(EstadoGafete),
    #[error("El gafete ya está prestado a otra persona")]
    Prestado,
}

impl ErrorPrestamoGafete {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::NoRegistrado => "gafete_no_registrado",
            Self::NoDisponible(_) => "gafete_no_disponible",
            Self::Prestado => "gafete_prestado",
        }
    }
}

/// E3: un gafete se puede prestar si existe, está disponible y nadie lo
/// tiene ahora.
pub const fn verificar_prestamo(situacion: SituacionGafete) -> Result<(), ErrorPrestamoGafete> {
    match situacion {
        SituacionGafete::NoRegistrado => Err(ErrorPrestamoGafete::NoRegistrado),
        SituacionGafete::Registrado {
            estado: EstadoGafete::Perdido,
            ..
        } => Err(ErrorPrestamoGafete::NoDisponible(EstadoGafete::Perdido)),
        SituacionGafete::Registrado {
            estado: EstadoGafete::DeBaja,
            ..
        } => Err(ErrorPrestamoGafete::NoDisponible(EstadoGafete::DeBaja)),
        SituacionGafete::Registrado { prestado: true, .. } => Err(ErrorPrestamoGafete::Prestado),
        SituacionGafete::Registrado {
            estado: EstadoGafete::Disponible,
            prestado: false,
        } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn numero(n: u32) -> NumeroGafete {
        NumeroGafete::nuevo(n).unwrap()
    }

    fn disponible() -> Gafete {
        Gafete::registrar(TipoGafete::Contratista, numero(25), false).unwrap()
    }

    fn portador() -> Portador {
        Portador::Contratista(ContratistaId::desde_uuid(Uuid::from_u128(7)))
    }

    #[test]
    fn el_numero_es_mayor_a_cero() {
        assert_eq!(NumeroGafete::nuevo(0), Err(ErrorGafete::NumeroInvalido));
        assert_eq!(numero(1).valor(), 1);
    }

    #[test]
    fn un_gafete_nuevo_nace_disponible_y_no_se_repite() {
        let gafete = disponible();
        assert_eq!(gafete.estado(), EstadoGafete::Disponible);
        assert_eq!(gafete.portador(), None);
        assert_eq!(
            Gafete::registrar(TipoGafete::Contratista, numero(25), true),
            Err(ErrorGafete::Repetido)
        );
    }

    #[test]
    fn el_rango_incluye_ambos_extremos_y_tiene_tope() {
        let numeros = Gafete::rango(5, 8).unwrap();
        assert_eq!(
            numeros.iter().map(|n| n.valor()).collect::<Vec<_>>(),
            [5, 6, 7, 8]
        );
        assert_eq!(Gafete::rango(3, 3).unwrap().len(), 1, "un solo número");
        assert_eq!(
            Gafete::rango(1, 1000).unwrap().len(),
            1000,
            "el tope exacto vale"
        );
        assert_eq!(Gafete::rango(1, 1001), Err(ErrorGafete::RangoInvalido));
        assert_eq!(Gafete::rango(8, 5), Err(ErrorGafete::RangoInvalido));
        assert_eq!(Gafete::rango(0, 5), Err(ErrorGafete::RangoInvalido));
    }

    #[test]
    fn perdido_y_resuelto_vuelve_a_estar_disponible() {
        let mut gafete = disponible();
        let perdido = gafete.marcar_perdido(portador()).unwrap();
        assert_eq!(gafete.estado(), EstadoGafete::Perdido);
        assert_eq!(gafete.portador(), Some(&portador()));
        let campos: Vec<_> = perdido.iter().map(|c| c.campo).collect();
        assert_eq!(campos, ["estado", "portador"], "auditoría del perdido");

        let resuelto = gafete.resolver(Resolucion::Pagado).unwrap();
        assert_eq!(gafete.estado(), EstadoGafete::Disponible);
        assert_eq!(gafete.portador(), None, "ya no tiene portador");
        assert_eq!(
            resuelto.last().unwrap().despues,
            "PAGADO",
            "queda el motivo"
        );
    }

    #[test]
    fn solo_un_gafete_disponible_se_marca_perdido_o_se_da_de_baja() {
        let mut perdido = disponible();
        perdido.marcar_perdido(portador()).unwrap();
        assert_eq!(
            perdido.marcar_perdido(portador()),
            Err(ErrorGafete::NoDisponible)
        );
        assert_eq!(perdido.dar_de_baja(false), Err(ErrorGafete::NoDisponible));

        let mut de_baja = disponible();
        de_baja.dar_de_baja(false).unwrap();
        assert_eq!(de_baja.estado(), EstadoGafete::DeBaja);
        assert_eq!(
            de_baja.marcar_perdido(portador()),
            Err(ErrorGafete::NoDisponible)
        );
    }

    #[test]
    fn solo_se_resuelve_un_gafete_perdido() {
        assert_eq!(
            disponible().resolver(Resolucion::Aparecido),
            Err(ErrorGafete::NoEstaPerdido)
        );
    }

    #[test]
    fn no_se_da_de_baja_un_gafete_prestado() {
        let mut gafete = disponible();
        assert_eq!(gafete.dar_de_baja(true), Err(ErrorGafete::EnUso));
        assert_eq!(
            gafete.estado(),
            EstadoGafete::Disponible,
            "quedó como estaba"
        );
    }

    #[test]
    fn prestar_exige_registrado_disponible_y_libre() {
        let libre = SituacionGafete::Registrado {
            estado: EstadoGafete::Disponible,
            prestado: false,
        };
        assert_eq!(verificar_prestamo(libre), Ok(()));
        assert_eq!(
            verificar_prestamo(SituacionGafete::NoRegistrado),
            Err(ErrorPrestamoGafete::NoRegistrado)
        );
        assert_eq!(
            verificar_prestamo(SituacionGafete::Registrado {
                estado: EstadoGafete::Perdido,
                prestado: false
            }),
            Err(ErrorPrestamoGafete::NoDisponible(EstadoGafete::Perdido))
        );
        assert_eq!(
            verificar_prestamo(SituacionGafete::Registrado {
                estado: EstadoGafete::Disponible,
                prestado: true
            }),
            Err(ErrorPrestamoGafete::Prestado)
        );
    }

    #[test]
    fn los_codigos_van_y_vuelven() {
        for tipo in TipoGafete::TODOS {
            assert_eq!(TipoGafete::desde_codigo(tipo.codigo()), Some(tipo));
        }
        for estado in [
            EstadoGafete::Disponible,
            EstadoGafete::Perdido,
            EstadoGafete::DeBaja,
        ] {
            assert_eq!(EstadoGafete::desde_codigo(estado.codigo()), Some(estado));
        }
    }

    #[test]
    fn el_ultimo_portador_corresponde_al_tipo_del_gafete() {
        let contratista = portador();
        let persona = Portador::Persona(Cedula::normalizar("111111111").unwrap());
        let kof = Portador::PersonalKof(PersonalKofId::desde_uuid(Uuid::from_u128(9)));
        let casos = [
            (TipoGafete::Contratista, &contratista, true),
            (TipoGafete::Contratista, &persona, false),
            (TipoGafete::Visita, &persona, true),
            (TipoGafete::Proveedor, &persona, true),
            (TipoGafete::Proveedor, &contratista, false),
            (TipoGafete::ProvisionalKof, &kof, true),
            (TipoGafete::ProvisionalKof, &persona, false),
            (TipoGafete::Visita, &kof, false),
        ];
        for (tipo, portador, corresponde) in casos {
            let mut gafete = Gafete::registrar(tipo, numero(1), false).unwrap();
            let resultado = gafete.marcar_perdido(portador.clone());
            if corresponde {
                assert!(resultado.is_ok(), "{tipo} con {portador}: {resultado:?}");
                assert_eq!(gafete.portador(), Some(portador));
            } else {
                assert_eq!(
                    resultado,
                    Err(ErrorGafete::PortadorDeOtroTipo),
                    "{tipo} con {portador}"
                );
                assert_eq!(gafete.estado(), EstadoGafete::Disponible, "no cambia nada");
            }
        }
    }

    #[test]
    fn la_cedula_del_portador_es_nacional_o_de_extranjero() {
        let mut gafete = Gafete::registrar(TipoGafete::Visita, numero(1), false).unwrap();
        let pasaporte = Portador::Persona(Cedula::normalizar("AB123").unwrap());
        assert_eq!(
            gafete.marcar_perdido(pasaporte),
            Err(ErrorGafete::CedulaDelPortadorInvalida)
        );
        assert_eq!(gafete.estado(), EstadoGafete::Disponible, "no cambia nada");
    }
}
