//! Casos de uso del ingreso por correo: entrada y salida de una visita
//! autorizada por correo.

use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, SituacionGafete, TipoGafete};
use limen_dominio::hecho::{Hecho, HechoId};
use limen_dominio::ingreso_correo::{
    BorradorEntradaCorreo, DatosEntradaCorreo, ErrorIngresoCorreo, HechosEntradaCorreo,
    IngresoCorreo, IngresoCorreoId, Motivo, problemas,
};
use limen_dominio::medio::TipoMedio;
use limen_dominio::movimiento::{ErrorSalida, Marca};
use limen_dominio::presencia::{Identidad, Via, YaEstaAdentro};
use limen_dominio::visitante::{Visitante, cedula_de};

use super::gafetes::situacion_para_prestar;
use super::hora::sellar;
use super::veto::cedula_vetada;
use crate::errores::ErrorCaso;
use crate::puertos::{
    FabricaUnidadDeTrabajo, GeneradorIds, RegistroHechos, Reloj, RepositorioGafetes,
    RepositorioIngresosCorreo, RepositorioPresencias, RepositorioReloj, Restriccion,
    UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorEntradaCorreo = ErrorCaso<ErrorIngresoCorreo>;
pub type ErrorSalidaCorreo = ErrorCaso<ErrorSalida>;

/// La base rechazó la entrada porque otro equipo registró a la misma
/// persona o prestó el mismo gafete primero.
fn conflicto_de_entrada(restriccion: Restriccion) -> Option<ErrorIngresoCorreo> {
    match restriccion {
        Restriccion::PresenciaPersona => Some(ErrorIngresoCorreo::YaEstaAdentro(YaEstaAdentro(
            Via::Correo,
        ))),
        Restriccion::GafetePrestado => {
            Some(ErrorIngresoCorreo::Gafete(ErrorPrestamoGafete::Prestado))
        }
        _ => None,
    }
}

/// Lo que llega del formulario de ingreso por correo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComandoEntradaCorreo {
    pub cedula: String,
    pub nombre: String,
    /// A quién visita o para qué viene.
    pub motivo: String,
    pub medio: TipoMedio,
    pub placa: Option<String>,
    /// Número del gafete de visita (obligatorio).
    pub gafete: u32,
}

#[derive(Debug)]
pub struct RegistrarEntradaCorreo<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarEntradaCorreo<F, R, G> {
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
        comando: &ComandoEntradaCorreo,
    ) -> Result<IngresoCorreoId, ErrorEntradaCorreo> {
        let visitante = Visitante::nuevo(&comando.cedula, &comando.nombre)
            .map_err(|error| ErrorCaso::Negocio(ErrorIngresoCorreo::Visitante(error)))?;
        let motivo = Motivo::nuevo(&comando.motivo)
            .map_err(|error| ErrorCaso::Negocio(ErrorIngresoCorreo::Motivo(error)))?;
        // Un gafete 0 no existe en ningún catálogo.
        let gafete = NumeroGafete::nuevo(comando.gafete).map_err(|_| {
            ErrorCaso::Negocio(ErrorIngresoCorreo::Gafete(
                ErrorPrestamoGafete::NoRegistrado,
            ))
        })?;

        let mut uow = self.fabrica.nueva();
        let cedula = visitante.cedula();
        let hechos = HechosEntradaCorreo {
            adentro_por: uow
                .presencias()
                .via_adentro(&Identidad::from(cedula))
                .await?,
            vetada: cedula_vetada(uow.contratistas(), cedula).await?,
            situacion_gafete: situacion_para_prestar(uow.gafetes(), TipoGafete::Visita, gafete)
                .await?,
        };
        let hora = sellar(&mut uow, &self.reloj).await?;
        let marca = Marca {
            en: hora.en,
            operador: sesion.operador(),
        };
        let datos = DatosEntradaCorreo {
            medio: comando.medio,
            placa: comando.placa.as_deref(),
            gafete,
        };
        let ingreso = IngresoCorreo::registrar_entrada(
            IngresoCorreoId::desde_uuid(self.ids.nuevo()),
            visitante,
            motivo,
            datos,
            hechos,
            marca,
        )
        .map_err(ErrorCaso::Negocio)?;

        uow.ingresos_correo().guardar(&ingreso);
        uow.hechos().anotar(Hecho::entrada_correo(
            HechoId::desde_uuid(self.ids.nuevo()),
            &ingreso,
            hora.into(),
        ));
        uow.presencias()
            .anotar_entrada(&Identidad::from(ingreso.cedula()), Via::Correo, marca.en);
        uow.gafetes()
            .anotar_prestamo(TipoGafete::Visita, gafete, marca.en);
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_entrada))?;
        Ok(ingreso.id())
    }
}

/// El formulario de ingreso por correo tal cual está mientras se llena: un
/// gafete 0 es "todavía no lo escribió".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComandoValidarEntradaCorreo {
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub medio: TipoMedio,
    pub placa: Option<String>,
    pub gafete: u32,
}

/// Dice de una vez todo lo que impide registrar el ingreso, para marcar cada
/// campo a la vez. Sólo lee: no registra nada. Averigua lo que hace falta (si
/// ya está adentro, veto, gafete) y el dominio decide.
#[derive(Debug)]
pub struct ValidarEntradaCorreo<F> {
    fabrica: F,
}

impl<F: FabricaUnidadDeTrabajo> ValidarEntradaCorreo<F> {
    pub const fn new(fabrica: F) -> Self {
        Self { fabrica }
    }

    pub async fn ejecutar(
        &self,
        comando: &ComandoValidarEntradaCorreo,
    ) -> Result<Vec<ErrorIngresoCorreo>, ErrorEntradaCorreo> {
        let mut uow = self.fabrica.nueva();
        // Sin cédula válida no hay a quién buscar: eso ya lo marca la cédula.
        let (adentro_por, vetada) = match cedula_de(&comando.cedula) {
            Ok(cedula) => (
                uow.presencias()
                    .via_adentro(&Identidad::from(&cedula))
                    .await?,
                cedula_vetada(uow.contratistas(), &cedula).await?,
            ),
            Err(_) => (None, false),
        };
        let gafete = NumeroGafete::nuevo(comando.gafete).ok();
        let situacion_gafete = match gafete {
            Some(numero) => {
                situacion_para_prestar(uow.gafetes(), TipoGafete::Visita, numero).await?
            }
            None => SituacionGafete::NoRegistrado,
        };
        let hechos = HechosEntradaCorreo {
            adentro_por,
            vetada,
            situacion_gafete,
        };
        let borrador = BorradorEntradaCorreo {
            cedula: &comando.cedula,
            nombre: &comando.nombre,
            motivo: &comando.motivo,
            medio: comando.medio,
            placa: comando.placa.as_deref(),
            gafete,
        };
        Ok(problemas(&borrador, hechos))
    }
}

/// Registra la salida de una visita por correo: por el ingreso o por el
/// número del gafete de visita que devuelve.
#[derive(Debug)]
pub struct RegistrarSalidaCorreo<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarSalidaCorreo<F, R, G> {
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
        id: IngresoCorreoId,
    ) -> Result<(), ErrorSalidaCorreo> {
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos_correo()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    /// Salida por el gafete de visita que devuelve la persona.
    pub async fn por_gafete(&self, sesion: &Sesion, numero: u32) -> Result<(), ErrorSalidaCorreo> {
        let numero = NumeroGafete::nuevo(numero).map_err(|_| ErrorCaso::NoEncontrado)?;
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos_correo()
            .abierto_con_gafete(numero)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    async fn cerrar<U: UnidadDeTrabajo>(
        &self,
        sesion: &Sesion,
        mut uow: U,
        mut ingreso: IngresoCorreo,
    ) -> Result<(), ErrorSalidaCorreo> {
        let hora = sellar(&mut uow, &self.reloj).await?;
        let marca = Marca {
            en: hora.en,
            operador: sesion.operador(),
        };
        ingreso
            .registrar_salida(marca)
            .map_err(ErrorCaso::Negocio)?;

        uow.ingresos_correo().guardar(&ingreso);
        if let Some(hecho) =
            Hecho::salida_correo(HechoId::desde_uuid(self.ids.nuevo()), &ingreso, hora.into())
        {
            uow.hechos().anotar(hecho);
        }
        uow.presencias()
            .anotar_salida(&Identidad::from(ingreso.cedula()));
        uow.gafetes()
            .anotar_devolucion(TipoGafete::Visita, ingreso.gafete());
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar().await.map_err(ErrorCaso::from)
    }
}
