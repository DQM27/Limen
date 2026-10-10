//! Casos de uso del catálogo de gafetes.

use limen_dominio::auditoria::CambioCampo;
use limen_dominio::gafete::{
    ErrorGafete, Gafete, NumeroGafete, Portador, Resolucion, SituacionGafete, TipoGafete,
};

use crate::errores::ErrorCaso;
use crate::puertos::{
    AccionAuditada, EntradaAuditoria, ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds,
    RegistroAuditado, RegistroAuditoria, Reloj, RepositorioGafetes, Restriccion, UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorGafetes = ErrorCaso<ErrorGafete>;

/// La base rechazó un número porque otro equipo lo creó primero.
fn conflicto_de_numero(restriccion: Restriccion) -> Option<ErrorGafete> {
    (restriccion == Restriccion::NumeroGafete).then_some(ErrorGafete::Repetido)
}

/// Lo que el dominio necesita saber de un gafete para decidir si se presta
/// (regla E3). Lo usan todas las vías de ingreso.
pub(crate) async fn situacion_para_prestar<G: RepositorioGafetes>(
    gafetes: &G,
    tipo: TipoGafete,
    numero: NumeroGafete,
) -> Result<SituacionGafete, ErrorPersistencia> {
    Ok(match gafetes.obtener(tipo, numero).await? {
        None => SituacionGafete::NoRegistrado,
        Some(gafete) => SituacionGafete::Registrado {
            estado: gafete.estado(),
            prestado: gafetes.prestado(tipo, numero).await?,
        },
    })
}

/// Crea un rango de gafetes de un tipo. Todo o nada: si alguno ya existe,
/// no se crea ninguno.
#[derive(Debug)]
pub struct RegistrarGafetes<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarGafetes<F, R, G> {
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
        tipo: TipoGafete,
        desde: u32,
        hasta: u32,
    ) -> Result<Vec<NumeroGafete>, ErrorGafetes> {
        let numeros = Gafete::rango(desde, hasta).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let existentes = uow.gafetes().existentes(tipo, &numeros).await?;
        let gafetes = numeros
            .iter()
            .map(|numero| Gafete::registrar(tipo, *numero, existentes.contains(numero)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ErrorCaso::Negocio)?;

        let ahora = self.reloj.ahora();
        for gafete in &gafetes {
            uow.gafetes().agregar(gafete);
            uow.auditoria().anotar(EntradaAuditoria::nueva(
                self.ids.nuevo(),
                RegistroAuditado::Gafete(tipo, gafete.numero()),
                AccionAuditada::Alta,
                gafete.cambios_de_alta(),
                sesion,
                ahora,
            ));
        }
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_numero))?;
        Ok(numeros)
    }
}

/// Qué le pasa a un gafete existente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CambioGafete {
    /// F4, F5.
    MarcarPerdido(Portador),
    /// F3.
    Resolver(Resolucion),
    /// F4, F6.
    DarDeBaja,
}

/// Cambia el estado de un gafete del catálogo y lo audita.
#[derive(Debug)]
pub struct CambiarGafete<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> CambiarGafete<F, R, G> {
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
        tipo: TipoGafete,
        numero: u32,
        cambio: CambioGafete,
    ) -> Result<Vec<CambioCampo>, ErrorGafetes> {
        let numero = NumeroGafete::nuevo(numero).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let mut gafete = uow
            .gafetes()
            .obtener(tipo, numero)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let cambios = match cambio {
            CambioGafete::MarcarPerdido(portador) => gafete.marcar_perdido(portador),
            CambioGafete::Resolver(resolucion) => gafete.resolver(resolucion),
            CambioGafete::DarDeBaja => {
                let en_uso = uow.gafetes().prestado(tipo, numero).await?;
                gafete.dar_de_baja(en_uso)
            }
        }
        .map_err(ErrorCaso::Negocio)?;

        uow.gafetes().actualizar(&gafete);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            RegistroAuditado::Gafete(tipo, numero),
            AccionAuditada::Edicion,
            cambios.clone(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar().await.map_err(ErrorCaso::from)?;
        Ok(cambios)
    }
}
