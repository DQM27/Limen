//! Ingreso de un proveedor (bloque H de `docs/reglas.md`).
//!
//! La persona no está en un catálogo: su cédula y su nombre llegan con cada
//! ingreso (ver [`crate::visitante`]). Al entrar se revisa, en este orden
//! (el primer motivo que falla es el que se informa):
//! 1. el medio: en vehículo la placa es obligatoria (E2);
//! 2. la empresa proveedora existe (H1);
//! 3. la persona no está adentro por ninguna vía (H3, A7). Pesa más que el
//!    veto: si está adentro, lo que corresponde es registrar su salida;
//! 4. la cédula no está vetada (A8);
//! 5. el gafete de proveedor: obligatorio, registrado, disponible y libre
//!    (H3, E3).
//!
//! El reloj no detiene la entrada (E5): la marca llega con la hora ya
//! sellada por [`crate::reloj::sellar_hora`].

use std::fmt;

use uuid::Uuid;

use crate::cedula::Cedula;
use crate::empresa_proveedora::EmpresaProveedoraId;
use crate::gafete::{ErrorPrestamoGafete, NumeroGafete, SituacionGafete, verificar_prestamo};
use crate::medio::{ErrorMedio, Medio, TipoMedio};
use crate::movimiento::{ErrorSalida, Marca, cerrar};
use crate::presencia::{Via, YaEstaAdentro, verificar_afuera};
use crate::visitante::{ErrorVisitante, PersonaVetada, Visitante, verificar_veto};

/// Identificador global de un ingreso de proveedor (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IngresoProveedorId(Uuid);

impl IngresoProveedorId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for IngresoProveedorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Lo que llega del formulario, además de la persona.
#[derive(Debug, Clone, Copy)]
pub struct DatosEntradaProveedor<'a> {
    pub empresa: EmpresaProveedoraId,
    pub medio: TipoMedio,
    pub placa: Option<&'a str>,
    /// Siempre obligatorio para un proveedor.
    pub gafete: NumeroGafete,
}

/// Lo que el caso de uso averiguó antes de pedir la decisión.
#[derive(Debug, Clone, Copy)]
pub struct HechosEntradaProveedor {
    pub empresa_existe: bool,
    /// Por qué vía está adentro la persona (por su cédula), si lo está.
    pub adentro_por: Option<Via>,
    /// La cédula tiene el acceso denegado (A8).
    pub vetada: bool,
    /// Situación del gafete de proveedor indicado.
    pub situacion_gafete: SituacionGafete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorIngresoProveedor {
    #[error("{0}")]
    Visitante(ErrorVisitante),
    #[error("{0}")]
    Medio(ErrorMedio),
    #[error("La empresa proveedora no existe")]
    EmpresaNoExiste,
    #[error("{0}")]
    YaEstaAdentro(YaEstaAdentro),
    #[error("{0}")]
    AccesoDenegado(PersonaVetada),
    #[error("{0}")]
    Gafete(ErrorPrestamoGafete),
}

impl ErrorIngresoProveedor {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Visitante(error) => error.codigo(),
            Self::Medio(error) => error.codigo(),
            Self::EmpresaNoExiste => "empresa_proveedora_no_existe",
            Self::YaEstaAdentro(error) => error.codigo(),
            Self::AccesoDenegado(error) => error.codigo(),
            Self::Gafete(error) => error.codigo(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoProveedor {
    id: IngresoProveedorId,
    visitante: Visitante,
    empresa: EmpresaProveedoraId,
    medio: Medio,
    gafete: NumeroGafete,
    entrada: Marca,
    salida: Option<Marca>,
}

/// Datos de un ingreso de proveedor guardado, para reconstruirlo.
#[derive(Debug, Clone)]
pub struct IngresoProveedorGuardado {
    pub id: IngresoProveedorId,
    pub visitante: Visitante,
    pub empresa: EmpresaProveedoraId,
    pub medio: Medio,
    pub gafete: NumeroGafete,
    pub entrada: Marca,
    pub salida: Option<Marca>,
}

impl IngresoProveedor {
    pub fn registrar_entrada(
        id: IngresoProveedorId,
        visitante: Visitante,
        datos: DatosEntradaProveedor<'_>,
        hechos: HechosEntradaProveedor,
        entrada: Marca,
    ) -> Result<Self, ErrorIngresoProveedor> {
        let medio = Medio::desde_formulario(datos.medio, datos.placa)
            .map_err(ErrorIngresoProveedor::Medio)?;
        if !hechos.empresa_existe {
            return Err(ErrorIngresoProveedor::EmpresaNoExiste);
        }
        verificar_afuera(hechos.adentro_por).map_err(ErrorIngresoProveedor::YaEstaAdentro)?;
        verificar_veto(hechos.vetada).map_err(ErrorIngresoProveedor::AccesoDenegado)?;
        verificar_prestamo(hechos.situacion_gafete).map_err(ErrorIngresoProveedor::Gafete)?;
        Ok(Self {
            id,
            visitante,
            empresa: datos.empresa,
            medio,
            gafete: datos.gafete,
            entrada,
            salida: None,
        })
    }

    /// Registra la salida (E4). El gafete queda libre con ella.
    pub fn registrar_salida(&mut self, salida: Marca) -> Result<(), ErrorSalida> {
        cerrar(self.entrada, &mut self.salida, salida)
    }

    pub fn restaurar(guardado: IngresoProveedorGuardado) -> Self {
        Self {
            id: guardado.id,
            visitante: guardado.visitante,
            empresa: guardado.empresa,
            medio: guardado.medio,
            gafete: guardado.gafete,
            entrada: guardado.entrada,
            salida: guardado.salida,
        }
    }

    pub const fn id(&self) -> IngresoProveedorId {
        self.id
    }

    pub const fn visitante(&self) -> &Visitante {
        &self.visitante
    }

    pub const fn cedula(&self) -> &Cedula {
        self.visitante.cedula()
    }

    pub const fn empresa(&self) -> EmpresaProveedoraId {
        self.empresa
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
    use crate::medio::Placa;
    use crate::operador::OperadorId;

    const ENTRADA: &str = "2026-10-09T14:00:00Z";

    fn marca(texto: &str) -> Marca {
        Marca {
            en: texto.parse().unwrap(),
            operador: OperadorId::desde_uuid(Uuid::from_u128(9)),
        }
    }

    fn visitante() -> Visitante {
        Visitante::nuevo("111111111", "ANA PEÑA").unwrap()
    }

    fn datos() -> DatosEntradaProveedor<'static> {
        DatosEntradaProveedor {
            empresa: EmpresaProveedoraId::desde_uuid(Uuid::from_u128(2)),
            medio: TipoMedio::Vehiculo,
            placa: Some("abc-123"),
            gafete: NumeroGafete::nuevo(4).unwrap(),
        }
    }

    const LIBRE: SituacionGafete = SituacionGafete::Registrado {
        estado: EstadoGafete::Disponible,
        prestado: false,
    };

    fn hechos() -> HechosEntradaProveedor {
        HechosEntradaProveedor {
            empresa_existe: true,
            adentro_por: None,
            vetada: false,
            situacion_gafete: LIBRE,
        }
    }

    fn entrar(
        datos: DatosEntradaProveedor<'_>,
        hechos: HechosEntradaProveedor,
    ) -> Result<IngresoProveedor, ErrorIngresoProveedor> {
        IngresoProveedor::registrar_entrada(
            IngresoProveedorId::desde_uuid(Uuid::from_u128(1)),
            visitante(),
            datos,
            hechos,
            marca(ENTRADA),
        )
    }

    #[test]
    fn entra_con_placa_y_gafete() {
        let ingreso = entrar(datos(), hechos()).unwrap();
        assert_eq!(ingreso.cedula().as_str(), "111111111");
        assert_eq!(
            ingreso.medio(),
            &Medio::Vehiculo(Placa::nueva("ABC-123").unwrap())
        );
        assert_eq!(ingreso.gafete(), NumeroGafete::nuevo(4).unwrap());
        assert_eq!(ingreso.entrada(), marca(ENTRADA));
        assert!(ingreso.esta_abierto(), "recién entra");
    }

    #[test]
    fn a_pie_descarta_la_placa() {
        let mut a_pie = datos();
        a_pie.medio = TipoMedio::APie;
        assert_eq!(entrar(a_pie, hechos()).unwrap().medio(), &Medio::APie);
    }

    #[test]
    fn en_vehiculo_exige_placa() {
        let mut sin_placa = datos();
        sin_placa.placa = None;
        assert_eq!(
            entrar(sin_placa, hechos()),
            Err(ErrorIngresoProveedor::Medio(ErrorMedio::PlacaRequerida))
        );
    }

    #[test]
    fn la_empresa_debe_existir() {
        let mut sin_empresa = hechos();
        sin_empresa.empresa_existe = false;
        assert_eq!(
            entrar(datos(), sin_empresa),
            Err(ErrorIngresoProveedor::EmpresaNoExiste)
        );
    }

    #[test]
    fn no_entra_si_ya_esta_adentro_por_cualquier_via() {
        for via in Via::TODAS {
            let mut adentro = hechos();
            adentro.adentro_por = Some(via);
            assert_eq!(
                entrar(datos(), adentro),
                Err(ErrorIngresoProveedor::YaEstaAdentro(YaEstaAdentro(via))),
                "adentro como {via}"
            );
        }
    }

    #[test]
    fn la_persona_vetada_no_entra() {
        let mut vetada = hechos();
        vetada.vetada = true;
        assert_eq!(
            entrar(datos(), vetada),
            Err(ErrorIngresoProveedor::AccesoDenegado(PersonaVetada))
        );
    }

    #[test]
    fn estar_adentro_pesa_mas_que_el_veto() {
        let mut ambos = hechos();
        ambos.vetada = true;
        ambos.adentro_por = Some(Via::Proveedor);
        assert_eq!(
            entrar(datos(), ambos),
            Err(ErrorIngresoProveedor::YaEstaAdentro(YaEstaAdentro(
                Via::Proveedor
            ))),
            "lo que corresponde es registrar la salida"
        );
    }

    #[test]
    fn el_gafete_debe_poder_prestarse() {
        let casos = [
            (
                SituacionGafete::NoRegistrado,
                ErrorPrestamoGafete::NoRegistrado,
            ),
            (
                SituacionGafete::Registrado {
                    estado: EstadoGafete::Perdido,
                    prestado: false,
                },
                ErrorPrestamoGafete::NoDisponible(EstadoGafete::Perdido),
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
                entrar(datos(), con_gafete),
                Err(ErrorIngresoProveedor::Gafete(error)),
                "{situacion:?}"
            );
        }
    }

    #[test]
    fn el_medio_se_revisa_antes_que_todo_lo_demas() {
        let mut sin_placa = datos();
        sin_placa.placa = None;
        let todo_mal = HechosEntradaProveedor {
            empresa_existe: false,
            adentro_por: Some(Via::Correo),
            vetada: true,
            situacion_gafete: SituacionGafete::NoRegistrado,
        };
        assert_eq!(
            entrar(sin_placa, todo_mal),
            Err(ErrorIngresoProveedor::Medio(ErrorMedio::PlacaRequerida))
        );
    }

    #[test]
    fn la_salida_sigue_las_reglas_comunes() {
        let mut ingreso = entrar(datos(), hechos()).unwrap();
        assert_eq!(
            ingreso.registrar_salida(marca("2026-10-09T13:00:00Z")),
            Err(ErrorSalida::AnteriorALaEntrada)
        );
        ingreso
            .registrar_salida(marca("2026-10-09T18:00:00Z"))
            .unwrap();
        assert!(!ingreso.esta_abierto(), "ya salió");
        assert_eq!(
            ingreso.registrar_salida(marca("2026-10-09T19:00:00Z")),
            Err(ErrorSalida::YaSalio)
        );
    }

    #[test]
    fn todo_error_tiene_codigo() {
        assert_eq!(
            ErrorIngresoProveedor::EmpresaNoExiste.codigo(),
            "empresa_proveedora_no_existe"
        );
        assert_eq!(
            ErrorIngresoProveedor::AccesoDenegado(PersonaVetada).codigo(),
            "sin_acceso"
        );
        assert_eq!(
            ErrorIngresoProveedor::Visitante(ErrorVisitante::CedulaVacia).codigo(),
            "cedula_vacia"
        );
    }
}
