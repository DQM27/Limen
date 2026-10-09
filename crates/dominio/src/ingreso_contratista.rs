//! Ingreso de un contratista (bloque E de `docs/reglas.md`).
//!
//! Al entrar se revisa, en este orden (el primer motivo que falla es el que
//! se informa):
//! 1. el medio: en vehículo la placa es obligatoria (E2);
//! 2. el reloj no retrocedió respecto al último movimiento (E5);
//! 3. la persona no está adentro por ninguna vía (E1, A7). Pesa más que el
//!    acceso: si está adentro, lo que corresponde es registrar su salida;
//! 4. el acceso: sin acceso o PRAIND vencido, no entra (D2, D4);
//! 5. el gafete, si corresponde y se indicó uno: registrado, disponible y
//!    libre (E3).
//!
//! El gafete es opcional aunque el tipo lo requiera: el operador puede
//! registrar la entrada "sin gafete" y queda dicho quién lo hizo. A quien no
//! le corresponde gafete (IN HOUSE) se le ignora el número indicado.

use std::fmt;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::acceso::{MotivoDenegacion, ResultadoAcceso, verificar_acceso};
use crate::cedula::Cedula;
use crate::contratista::{Contratista, ContratistaId};
use crate::gafete::{ErrorPrestamoGafete, NumeroGafete, SituacionGafete, verificar_prestamo};
use crate::medio::{ErrorMedio, Medio, TipoMedio};
use crate::movimiento::{ErrorSalida, Marca, cerrar};
use crate::presencia::{Via, YaEstaAdentro, verificar_afuera};
use crate::reloj::{RelojAtrasado, verificar_reloj};

/// Identificador global de un ingreso (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IngresoId(Uuid);

impl IngresoId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for IngresoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Lo que llega del formulario de entrada.
#[derive(Debug, Clone, Copy)]
pub struct DatosEntrada<'a> {
    pub medio: TipoMedio,
    pub placa: Option<&'a str>,
    /// `None` = "sin gafete".
    pub gafete: Option<NumeroGafete>,
}

/// Lo que el caso de uso averiguó antes de pedir la decisión.
#[derive(Debug, Clone, Copy)]
pub struct HechosEntrada {
    pub ultimo_movimiento: Option<DateTime<Utc>>,
    /// Por qué vía está adentro la persona (por su cédula), si lo está.
    pub adentro_por: Option<Via>,
    /// Situación del gafete de [`gafete_que_aplica`]. Si el caso de uso no
    /// la averiguó, se trata como no registrado: nunca se presta a ciegas.
    pub situacion_gafete: Option<SituacionGafete>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorIngreso {
    #[error("{0}")]
    Medio(ErrorMedio),
    #[error("{0}")]
    Reloj(RelojAtrasado),
    #[error("{0}")]
    YaEstaAdentro(YaEstaAdentro),
    #[error("{0}")]
    AccesoDenegado(MotivoDenegacion),
    #[error("{0}")]
    Gafete(ErrorPrestamoGafete),
}

impl ErrorIngreso {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Medio(error) => error.codigo(),
            Self::Reloj(error) => error.codigo(),
            Self::YaEstaAdentro(error) => error.codigo(),
            Self::AccesoDenegado(motivo) => motivo.codigo(),
            Self::Gafete(error) => error.codigo(),
        }
    }
}

/// El gafete que de verdad se va a prestar: el indicado, sólo si al
/// contratista le corresponde gafete (PRAIND sí, IN HOUSE no).
pub fn gafete_que_aplica(
    contratista: &Contratista,
    indicado: Option<NumeroGafete>,
) -> Option<NumeroGafete> {
    indicado.filter(|_| contratista.requiere_gafete())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoContratista {
    id: IngresoId,
    contratista: ContratistaId,
    cedula: Cedula,
    medio: Medio,
    gafete: Option<NumeroGafete>,
    entrada: Marca,
    salida: Option<Marca>,
}

/// Una entrada aceptada y el resultado del acceso, para avisar al operador
/// si el PRAIND está por vencer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntradaRegistrada {
    pub ingreso: IngresoContratista,
    pub acceso: ResultadoAcceso,
}

/// Datos de un ingreso guardado, para reconstruirlo.
#[derive(Debug, Clone)]
pub struct IngresoGuardado {
    pub id: IngresoId,
    pub contratista: ContratistaId,
    pub cedula: Cedula,
    pub medio: Medio,
    pub gafete: Option<NumeroGafete>,
    pub entrada: Marca,
    pub salida: Option<Marca>,
}

impl IngresoContratista {
    pub fn registrar_entrada(
        id: IngresoId,
        contratista: &Contratista,
        datos: DatosEntrada<'_>,
        hechos: HechosEntrada,
        entrada: Marca,
        hoy: NaiveDate,
    ) -> Result<EntradaRegistrada, ErrorIngreso> {
        let medio =
            Medio::desde_formulario(datos.medio, datos.placa).map_err(ErrorIngreso::Medio)?;
        verificar_reloj(entrada.en, hechos.ultimo_movimiento).map_err(ErrorIngreso::Reloj)?;
        verificar_afuera(hechos.adentro_por).map_err(ErrorIngreso::YaEstaAdentro)?;
        let acceso = verificar_acceso(contratista, hoy);
        if let ResultadoAcceso::Denegado(motivo) = acceso {
            return Err(ErrorIngreso::AccesoDenegado(motivo));
        }
        let gafete = gafete_que_aplica(contratista, datos.gafete);
        if gafete.is_some() {
            verificar_prestamo(
                hechos
                    .situacion_gafete
                    .unwrap_or(SituacionGafete::NoRegistrado),
            )
            .map_err(ErrorIngreso::Gafete)?;
        }
        Ok(EntradaRegistrada {
            ingreso: Self {
                id,
                contratista: contratista.id(),
                cedula: contratista.cedula().clone(),
                medio,
                gafete,
                entrada,
                salida: None,
            },
            acceso,
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

    pub fn restaurar(guardado: IngresoGuardado) -> Self {
        Self {
            id: guardado.id,
            contratista: guardado.contratista,
            cedula: guardado.cedula,
            medio: guardado.medio,
            gafete: guardado.gafete,
            entrada: guardado.entrada,
            salida: guardado.salida,
        }
    }

    pub const fn id(&self) -> IngresoId {
        self.id
    }

    pub const fn contratista(&self) -> ContratistaId {
        self.contratista
    }

    pub const fn cedula(&self) -> &Cedula {
        &self.cedula
    }

    pub const fn medio(&self) -> &Medio {
        &self.medio
    }

    pub const fn gafete(&self) -> Option<NumeroGafete> {
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
    use crate::contratista::ContratistaGuardado;
    use crate::empresa::EmpresaId;
    use crate::gafete::EstadoGafete;
    use crate::medio::Placa;
    use crate::nombre::NombrePersona;
    use crate::operador::OperadorId;
    use crate::tipo_ingreso::TipoIngreso;

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    fn hoy() -> NaiveDate {
        fecha("2026-10-09")
    }

    fn marca(texto: &str) -> Marca {
        Marca {
            en: texto.parse().unwrap(),
            operador: OperadorId::desde_uuid(Uuid::from_u128(900)),
        }
    }

    fn entrada() -> Marca {
        marca("2026-10-09T14:00:00Z")
    }

    fn contratista(tipo: TipoIngreso, vence: &str, tiene_acceso: bool) -> Contratista {
        Contratista::restaurar(ContratistaGuardado {
            id: ContratistaId::desde_uuid(Uuid::from_u128(1)),
            cedula: Cedula::normalizar("111111111").unwrap(),
            nombre: NombrePersona::nuevo("ANA").unwrap(),
            empresa: EmpresaId::desde_uuid(Uuid::from_u128(2)),
            tipo_ingreso: tipo,
            fecha_vencimiento_praind: fecha(vence),
            tiene_acceso,
        })
    }

    fn praind() -> Contratista {
        contratista(TipoIngreso::Praind, "2027-01-01", true)
    }

    fn numero(n: u32) -> NumeroGafete {
        NumeroGafete::nuevo(n).unwrap()
    }

    const LIBRE: SituacionGafete = SituacionGafete::Registrado {
        estado: EstadoGafete::Disponible,
        prestado: false,
    };

    fn a_pie_con_gafete(n: u32) -> DatosEntrada<'static> {
        DatosEntrada {
            medio: TipoMedio::APie,
            placa: None,
            gafete: Some(numero(n)),
        }
    }

    const fn hechos() -> HechosEntrada {
        HechosEntrada {
            ultimo_movimiento: None,
            adentro_por: None,
            situacion_gafete: Some(LIBRE),
        }
    }

    fn entrar(
        contratista: &Contratista,
        datos: DatosEntrada<'_>,
        hechos: HechosEntrada,
    ) -> Result<EntradaRegistrada, ErrorIngreso> {
        IngresoContratista::registrar_entrada(
            IngresoId::desde_uuid(Uuid::from_u128(50)),
            contratista,
            datos,
            hechos,
            entrada(),
            hoy(),
        )
    }

    #[test]
    fn registra_la_entrada_con_gafete() {
        let registrada = entrar(&praind(), a_pie_con_gafete(25), hechos()).unwrap();
        let ingreso = registrada.ingreso;
        assert_eq!(ingreso.gafete(), Some(numero(25)));
        assert_eq!(ingreso.cedula().as_str(), "111111111", "para la presencia");
        assert_eq!(ingreso.entrada(), entrada());
        assert!(ingreso.esta_abierto(), "recién entró");
        assert_eq!(registrada.acceso, ResultadoAcceso::Permitido);
    }

    #[test]
    fn en_vehiculo_guarda_la_placa() {
        let datos = DatosEntrada {
            medio: TipoMedio::Vehiculo,
            placa: Some("abc 123"),
            gafete: None,
        };
        let ingreso = entrar(&praind(), datos, hechos()).unwrap().ingreso;
        assert_eq!(ingreso.medio().placa().map(Placa::as_str), Some("ABC 123"));
    }

    #[test]
    fn sin_gafete_tambien_puede_entrar() {
        let datos = DatosEntrada {
            medio: TipoMedio::APie,
            placa: None,
            gafete: None,
        };
        let sin_situacion = HechosEntrada {
            situacion_gafete: None,
            ..hechos()
        };
        let ingreso = entrar(&praind(), datos, sin_situacion).unwrap().ingreso;
        assert_eq!(ingreso.gafete(), None, "queda registrado sin gafete");
    }

    #[test]
    fn a_in_house_se_le_ignora_el_gafete() {
        let in_house = contratista(TipoIngreso::InHouse, "2027-01-01", true);
        let ingreso = entrar(&in_house, a_pie_con_gafete(25), hechos())
            .unwrap()
            .ingreso;
        assert_eq!(ingreso.gafete(), None, "su credencial ya es el gafete");
    }

    #[test]
    fn avisa_si_el_praind_esta_por_vencer() {
        let por_vencer = contratista(TipoIngreso::Praind, "2026-10-20", true);
        assert_eq!(
            entrar(&por_vencer, a_pie_con_gafete(25), hechos())
                .unwrap()
                .acceso,
            ResultadoAcceso::PermitidoConAdvertencia {
                dias_para_vencer: 11
            }
        );
    }

    #[test]
    fn informa_el_primer_motivo_en_orden() {
        let vehiculo_sin_placa = DatosEntrada {
            medio: TipoMedio::Vehiculo,
            placa: None,
            gafete: None,
        };
        let todo_mal = HechosEntrada {
            ultimo_movimiento: Some("2026-10-09T15:00:00Z".parse().unwrap()),
            adentro_por: Some(Via::Proveedor),
            situacion_gafete: Some(SituacionGafete::NoRegistrado),
        };
        let sin_acceso = contratista(TipoIngreso::Praind, "2020-01-01", false);

        assert_eq!(
            entrar(&sin_acceso, vehiculo_sin_placa, todo_mal),
            Err(ErrorIngreso::Medio(ErrorMedio::PlacaRequerida)),
            "1. el medio"
        );
        assert_eq!(
            entrar(&sin_acceso, a_pie_con_gafete(25), todo_mal),
            Err(ErrorIngreso::Reloj(RelojAtrasado)),
            "2. el reloj"
        );
        let reloj_bien = HechosEntrada {
            ultimo_movimiento: None,
            ..todo_mal
        };
        assert_eq!(
            entrar(&sin_acceso, a_pie_con_gafete(25), reloj_bien),
            Err(ErrorIngreso::YaEstaAdentro(YaEstaAdentro(Via::Proveedor))),
            "3. ya está adentro pesa más que el acceso"
        );
        let afuera = HechosEntrada {
            adentro_por: None,
            ..reloj_bien
        };
        assert_eq!(
            entrar(&sin_acceso, a_pie_con_gafete(25), afuera),
            Err(ErrorIngreso::AccesoDenegado(MotivoDenegacion::SinAcceso)),
            "4. el acceso"
        );
        assert_eq!(
            entrar(&praind(), a_pie_con_gafete(25), afuera),
            Err(ErrorIngreso::Gafete(ErrorPrestamoGafete::NoRegistrado)),
            "5. el gafete"
        );
    }

    #[test]
    fn con_el_praind_vencido_no_entra() {
        let vencido = contratista(TipoIngreso::InHouse, "2026-10-08", true);
        assert_eq!(
            entrar(&vencido, a_pie_con_gafete(1), hechos()),
            Err(ErrorIngreso::AccesoDenegado(
                MotivoDenegacion::PraindVencido
            ))
        );
    }

    #[test]
    fn un_gafete_prestado_o_perdido_no_se_presta() {
        let prestado = HechosEntrada {
            situacion_gafete: Some(SituacionGafete::Registrado {
                estado: EstadoGafete::Disponible,
                prestado: true,
            }),
            ..hechos()
        };
        assert_eq!(
            entrar(&praind(), a_pie_con_gafete(25), prestado),
            Err(ErrorIngreso::Gafete(ErrorPrestamoGafete::Prestado))
        );
        let perdido = HechosEntrada {
            situacion_gafete: Some(SituacionGafete::Registrado {
                estado: EstadoGafete::Perdido,
                prestado: false,
            }),
            ..hechos()
        };
        assert_eq!(
            entrar(&praind(), a_pie_con_gafete(25), perdido),
            Err(ErrorIngreso::Gafete(ErrorPrestamoGafete::NoDisponible(
                EstadoGafete::Perdido
            )))
        );
    }

    #[test]
    fn si_no_se_averiguo_el_gafete_no_se_presta_a_ciegas() {
        let sin_situacion = HechosEntrada {
            situacion_gafete: None,
            ..hechos()
        };
        assert_eq!(
            entrar(&praind(), a_pie_con_gafete(25), sin_situacion),
            Err(ErrorIngreso::Gafete(ErrorPrestamoGafete::NoRegistrado))
        );
    }

    #[test]
    fn la_salida_cierra_el_ingreso() {
        let mut ingreso = entrar(&praind(), a_pie_con_gafete(25), hechos())
            .unwrap()
            .ingreso;
        let salida = marca("2026-10-09T22:00:00Z");
        assert_eq!(ingreso.registrar_salida(salida, Some(entrada().en)), Ok(()));
        assert_eq!(ingreso.salida(), Some(salida));
        assert!(!ingreso.esta_abierto(), "ya salió");
        assert_eq!(
            ingreso.registrar_salida(salida, None),
            Err(ErrorSalida::YaSalio),
            "no sale dos veces"
        );
    }

    #[test]
    fn cada_error_tiene_codigo_estable() {
        assert_eq!(
            ErrorIngreso::YaEstaAdentro(YaEstaAdentro(Via::Correo)).codigo(),
            "ya_esta_adentro"
        );
        assert_eq!(
            ErrorIngreso::AccesoDenegado(MotivoDenegacion::SinAcceso).codigo(),
            "sin_acceso"
        );
    }
}
