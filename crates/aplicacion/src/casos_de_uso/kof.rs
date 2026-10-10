//! Casos de uso del personal KOF: su catálogo y el préstamo del gafete
//! provisional.

use limen_dominio::auditoria::CambioCampo;
use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, TipoGafete};
use limen_dominio::hecho::{Hecho, HechoId};
use limen_dominio::movimiento::Marca;
use limen_dominio::personal_kof::{
    CodigoEmpleado, ErrorPersonalKof, HechosPersonalKof, PersonalKof, PersonalKofId,
};
use limen_dominio::presencia::Via;
use limen_dominio::prestamo_kof::{
    ErrorDevolucionKof, ErrorPrestamoKof, HechosEntregaKof, PrestamoKof, PrestamoKofId,
};

use super::gafetes::situacion_para_prestar;
use crate::errores::ErrorCaso;
use crate::puertos::{
    AccionAuditada, EntradaAuditoria, FabricaUnidadDeTrabajo, GeneradorIds, RegistroAuditado,
    RegistroAuditoria, RegistroHechos, Reloj, RepositorioGafetes, RepositorioPersonalKof,
    RepositorioPresencias, RepositorioPrestamosKof, RepositorioReloj, Restriccion, UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorPersonal = ErrorCaso<ErrorPersonalKof>;
pub type ErrorEntregaKof = ErrorCaso<ErrorPrestamoKof>;
pub type ErrorDevolucion = ErrorCaso<ErrorDevolucionKof>;

// --- Catálogo ---

/// La base rechazó el código porque otro equipo lo guardó primero.
fn conflicto_de_codigo(restriccion: Restriccion) -> Option<ErrorPersonalKof> {
    (restriccion == Restriccion::CodigoEmpleado).then_some(ErrorPersonalKof::CodigoRepetido)
}

#[derive(Debug)]
pub struct RegistrarPersonalKof<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarPersonalKof<F, R, G> {
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
        codigo: &str,
        nombre: &str,
    ) -> Result<PersonalKofId, ErrorPersonal> {
        let codigo_normalizado = CodigoEmpleado::nuevo(codigo).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let hechos = HechosPersonalKof {
            codigo_en_uso: uow
                .personal_kof()
                .codigo_en_uso(&codigo_normalizado, None)
                .await?,
        };
        let id = PersonalKofId::desde_uuid(self.ids.nuevo());
        let persona =
            PersonalKof::registrar(id, codigo, nombre, hechos).map_err(ErrorCaso::Negocio)?;

        uow.personal_kof().guardar(&persona);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            RegistroAuditado::PersonalKof(id),
            AccionAuditada::Alta,
            persona.cambios_de_alta(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_codigo))?;
        Ok(id)
    }
}

/// Cambia el nombre de una persona o la activa o desactiva.
#[derive(Debug)]
pub struct EditarPersonalKof<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> EditarPersonalKof<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    /// Devuelve lo que cambió (vacío si todo era igual: entonces no se
    /// escribe nada).
    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        id: PersonalKofId,
        nombre: &str,
        activo: bool,
    ) -> Result<Vec<CambioCampo>, ErrorPersonal> {
        let mut uow = self.fabrica.nueva();
        let mut persona = uow
            .personal_kof()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let cambios = persona.editar(nombre, activo).map_err(ErrorCaso::Negocio)?;
        if cambios.is_empty() {
            return Ok(cambios);
        }

        uow.personal_kof().guardar(&persona);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            RegistroAuditado::PersonalKof(id),
            AccionAuditada::Edicion,
            cambios.clone(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar().await.map_err(ErrorCaso::from)?;
        Ok(cambios)
    }
}

// --- Gafete provisional ---

/// La base rechazó la entrega porque otro equipo prestó el mismo gafete o
/// le dio otro provisional a la misma persona primero.
fn conflicto_de_entrega(restriccion: Restriccion) -> Option<ErrorPrestamoKof> {
    match restriccion {
        // Tener el provisional y estar adentro es lo mismo para el KOF.
        Restriccion::PersonalKofConPrestamo | Restriccion::PresenciaPersona => {
            Some(ErrorPrestamoKof::YaTienePrestamo)
        }
        Restriccion::GafetePrestado => {
            Some(ErrorPrestamoKof::Gafete(ErrorPrestamoGafete::Prestado))
        }
        _ => None,
    }
}

#[derive(Debug)]
pub struct EntregarGafeteKof<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> EntregarGafeteKof<F, R, G> {
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
        personal: PersonalKofId,
        gafete: u32,
    ) -> Result<PrestamoKofId, ErrorEntregaKof> {
        // Un gafete 0 no existe en ningún inventario.
        let gafete = NumeroGafete::nuevo(gafete).map_err(|_| {
            ErrorCaso::Negocio(ErrorPrestamoKof::Gafete(ErrorPrestamoGafete::NoRegistrado))
        })?;
        let mut uow = self.fabrica.nueva();
        let persona = uow
            .personal_kof()
            .obtener(personal)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let hechos = HechosEntregaKof {
            ultimo_movimiento: uow.reloj().ultimo_movimiento().await?,
            ya_tiene_prestamo: uow.prestamos_kof().tiene_abierto(personal).await?,
            situacion_gafete: situacion_para_prestar(
                uow.gafetes(),
                TipoGafete::ProvisionalKof,
                gafete,
            )
            .await?,
        };
        let marca = Marca {
            en: self.reloj.ahora(),
            operador: sesion.operador(),
        };
        let prestamo = PrestamoKof::entregar(
            PrestamoKofId::desde_uuid(self.ids.nuevo()),
            &persona,
            gafete,
            hechos,
            marca,
        )
        .map_err(ErrorCaso::Negocio)?;

        // Entregar el gafete es la entrada: queda adentro, por la vía KOF.
        uow.prestamos_kof().anotar_entrega(&prestamo);
        uow.hechos().anotar(Hecho::entrega_kof(
            HechoId::desde_uuid(self.ids.nuevo()),
            &prestamo,
        ));
        uow.presencias()
            .anotar_entrada(&prestamo.identidad(), Via::Kof, marca.en);
        uow.gafetes()
            .anotar_prestamo(TipoGafete::ProvisionalKof, gafete, marca.en);
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_entrega))?;
        Ok(prestamo.id())
    }
}

/// Registra la devolución de un provisional: por el préstamo o por el
/// número del gafete que devuelve la persona.
#[derive(Debug)]
pub struct DevolverGafeteKof<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> DevolverGafeteKof<F, R, G> {
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
        id: PrestamoKofId,
    ) -> Result<(), ErrorDevolucion> {
        let mut uow = self.fabrica.nueva();
        let prestamo = uow
            .prestamos_kof()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, prestamo).await
    }

    /// Devolución por el número del gafete provisional.
    pub async fn por_gafete(&self, sesion: &Sesion, numero: u32) -> Result<(), ErrorDevolucion> {
        let numero = NumeroGafete::nuevo(numero).map_err(|_| ErrorCaso::NoEncontrado)?;
        let mut uow = self.fabrica.nueva();
        let prestamo = uow
            .prestamos_kof()
            .abierto_con_gafete(numero)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, prestamo).await
    }

    async fn cerrar<U: UnidadDeTrabajo>(
        &self,
        sesion: &Sesion,
        mut uow: U,
        mut prestamo: PrestamoKof,
    ) -> Result<(), ErrorDevolucion> {
        let marca = Marca {
            en: self.reloj.ahora(),
            operador: sesion.operador(),
        };
        let ultimo_movimiento = uow.reloj().ultimo_movimiento().await?;
        prestamo
            .devolver(marca, ultimo_movimiento)
            .map_err(ErrorCaso::Negocio)?;

        // Devolver el gafete es la salida.
        uow.prestamos_kof().anotar_devolucion(&prestamo);
        if let Some(hecho) = Hecho::devolucion_kof(HechoId::desde_uuid(self.ids.nuevo()), &prestamo)
        {
            uow.hechos().anotar(hecho);
        }
        uow.presencias().anotar_salida(&prestamo.identidad());
        uow.gafetes()
            .anotar_devolucion(TipoGafete::ProvisionalKof, prestamo.gafete());
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar().await.map_err(ErrorCaso::from)
    }
}
