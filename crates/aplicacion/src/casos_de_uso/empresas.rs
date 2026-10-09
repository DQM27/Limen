//! Casos de uso de empresas.

use limen_dominio::auditoria::CambioCampo;
use limen_dominio::empresa::{Empresa, EmpresaId, ErrorEmpresa, HechosEmpresa, NombreEmpresa};

use crate::errores::ErrorCaso;
use crate::puertos::{
    AccionAuditada, EntidadAuditada, EntradaAuditoria, FabricaUnidadDeTrabajo, GeneradorIds,
    RegistroAuditoria, Reloj, RepositorioEmpresas, Restriccion, UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorEmpresas = ErrorCaso<ErrorEmpresa>;

/// La base rechazó el nombre porque otro equipo lo guardó primero.
fn conflicto_de_nombre(restriccion: Restriccion) -> Option<ErrorEmpresa> {
    (restriccion == Restriccion::NombreEmpresa).then_some(ErrorEmpresa::NombreRepetido)
}

#[derive(Debug)]
pub struct RegistrarEmpresa<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarEmpresa<F, R, G> {
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
        nombre: &str,
    ) -> Result<EmpresaId, ErrorEmpresas> {
        let nombre_normalizado = NombreEmpresa::nuevo(nombre).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let hechos = HechosEmpresa {
            nombre_en_uso: uow
                .empresas()
                .nombre_en_uso(&nombre_normalizado, None)
                .await?,
        };
        let id = EmpresaId::desde_uuid(self.ids.nuevo());
        let empresa = Empresa::registrar(id, nombre, hechos).map_err(ErrorCaso::Negocio)?;

        uow.empresas().guardar(&empresa);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            EntidadAuditada::Empresa,
            id.uuid(),
            AccionAuditada::Alta,
            empresa.cambios_de_alta(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_nombre))?;
        Ok(id)
    }
}

#[derive(Debug)]
pub struct RenombrarEmpresa<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RenombrarEmpresa<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    /// Devuelve lo que cambió (vacío si el nombre ya era ese: entonces no se
    /// escribe nada).
    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        id: EmpresaId,
        nombre: &str,
    ) -> Result<Vec<CambioCampo>, ErrorEmpresas> {
        let nombre_normalizado = NombreEmpresa::nuevo(nombre).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let mut empresa = uow
            .empresas()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let hechos = HechosEmpresa {
            nombre_en_uso: uow
                .empresas()
                .nombre_en_uso(&nombre_normalizado, Some(id))
                .await?,
        };
        let cambios = empresa
            .renombrar(nombre, hechos)
            .map_err(ErrorCaso::Negocio)?;
        if cambios.is_empty() {
            return Ok(cambios);
        }

        uow.empresas().guardar(&empresa);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            EntidadAuditada::Empresa,
            id.uuid(),
            AccionAuditada::Edicion,
            cambios.clone(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_nombre))?;
        Ok(cambios)
    }
}
