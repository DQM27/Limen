//! Casos de uso de ingresos y salidas de contratistas.

use limen_dominio::acceso::ResultadoAcceso;
use limen_dominio::contratista::ContratistaId;
use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, TipoGafete};
use limen_dominio::ingreso_contratista::{
    DatosEntrada, ErrorIngreso, HechosEntrada, IngresoContratista, IngresoId, gafete_que_aplica,
};
use limen_dominio::medio::TipoMedio;
use limen_dominio::movimiento::{ErrorSalida, Marca};
use limen_dominio::presencia::{Via, YaEstaAdentro};

use super::gafetes::situacion_para_prestar;
use crate::errores::ErrorCaso;
use crate::puertos::{
    FabricaUnidadDeTrabajo, GeneradorIds, Reloj, RepositorioContratistas, RepositorioGafetes,
    RepositorioIngresos, RepositorioPresencias, RepositorioReloj, Restriccion, UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorEntrada = ErrorCaso<ErrorIngreso>;
pub type ErrorDeSalida = ErrorCaso<ErrorSalida>;

/// La base rechazó la entrada porque otro equipo registró a la misma
/// persona o prestó el mismo gafete primero.
fn conflicto_de_entrada(restriccion: Restriccion) -> Option<ErrorIngreso> {
    match restriccion {
        Restriccion::PresenciaPersona => {
            Some(ErrorIngreso::YaEstaAdentro(YaEstaAdentro(Via::Contratista)))
        }
        Restriccion::GafetePrestado => Some(ErrorIngreso::Gafete(ErrorPrestamoGafete::Prestado)),
        _ => None,
    }
}

/// Lo que llega del formulario de entrada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComandoEntrada {
    pub contratista: ContratistaId,
    pub medio: TipoMedio,
    pub placa: Option<String>,
    /// Número del gafete; `None` = "sin gafete".
    pub gafete: Option<u32>,
}

/// La entrada registrada y el resultado del acceso (para avisar si el
/// PRAIND está por vencer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaContratista {
    pub ingreso: IngresoId,
    pub acceso: ResultadoAcceso,
}

#[derive(Debug)]
pub struct RegistrarEntrada<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarEntrada<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        comando: &ComandoEntrada,
    ) -> Result<EntradaContratista, ErrorEntrada> {
        let mut uow = self.fabrica.nueva();
        let contratista = uow
            .contratistas()
            .obtener(comando.contratista)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        // Un gafete 0 no existe en ningún catálogo.
        let gafete = comando
            .gafete
            .map(NumeroGafete::nuevo)
            .transpose()
            .map_err(|_| {
                ErrorCaso::Negocio(ErrorIngreso::Gafete(ErrorPrestamoGafete::NoRegistrado))
            })?;

        let situacion_gafete = match gafete_que_aplica(&contratista, gafete) {
            Some(numero) => {
                Some(situacion_para_prestar(uow.gafetes(), TipoGafete::Contratista, numero).await?)
            }
            None => None,
        };
        let hechos = HechosEntrada {
            ultimo_movimiento: uow.reloj().ultimo_movimiento().await?,
            adentro_por: uow.presencias().via_adentro(contratista.cedula()).await?,
            situacion_gafete,
        };
        let marca = Marca {
            en: self.reloj.ahora(),
            operador: sesion.operador(),
        };
        let datos = DatosEntrada {
            medio: comando.medio,
            placa: comando.placa.as_deref(),
            gafete,
        };
        let registrada = IngresoContratista::registrar_entrada(
            IngresoId::desde_uuid(self.ids.nuevo()),
            &contratista,
            datos,
            hechos,
            marca,
            self.reloj.hoy(),
        )
        .map_err(ErrorCaso::Negocio)?;
        let ingreso = registrada.ingreso;

        uow.ingresos().guardar(&ingreso);
        uow.presencias()
            .anotar_entrada(ingreso.cedula(), Via::Contratista, marca.en);
        if let Some(numero) = ingreso.gafete() {
            uow.gafetes()
                .anotar_prestamo(TipoGafete::Contratista, numero, marca.en);
        }
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_entrada))?;
        Ok(EntradaContratista {
            ingreso: ingreso.id(),
            acceso: registrada.acceso,
        })
    }
}

/// Registra la salida de un contratista: por el ingreso (desde la lista de
/// quienes están adentro) o por el número del gafete que devuelve.
#[derive(Debug)]
pub struct RegistrarSalida<F, R> {
    fabrica: F,
    reloj: R,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj> RegistrarSalida<F, R> {
    pub const fn new(fabrica: F, reloj: R) -> Self {
        Self { fabrica, reloj }
    }

    pub async fn ejecutar(&self, sesion: &Sesion, id: IngresoId) -> Result<(), ErrorDeSalida> {
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    /// Salida por el gafete que devuelve la persona.
    pub async fn por_gafete(&self, sesion: &Sesion, numero: u32) -> Result<(), ErrorDeSalida> {
        let numero = NumeroGafete::nuevo(numero).map_err(|_| ErrorCaso::NoEncontrado)?;
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos()
            .abierto_con_gafete(numero)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    async fn cerrar<U: UnidadDeTrabajo>(
        &self,
        sesion: &Sesion,
        mut uow: U,
        mut ingreso: IngresoContratista,
    ) -> Result<(), ErrorDeSalida> {
        let ultimo_movimiento = uow.reloj().ultimo_movimiento().await?;
        let marca = Marca {
            en: self.reloj.ahora(),
            operador: sesion.operador(),
        };
        ingreso
            .registrar_salida(marca, ultimo_movimiento)
            .map_err(ErrorCaso::Negocio)?;

        uow.ingresos().guardar(&ingreso);
        uow.presencias().anotar_salida(ingreso.cedula());
        if let Some(numero) = ingreso.gafete() {
            uow.gafetes()
                .anotar_devolucion(TipoGafete::Contratista, numero);
        }
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar().await.map_err(ErrorCaso::from)
    }
}
