//! Casos de uso de contratistas.

use chrono::NaiveDate;
use limen_dominio::acceso::{ResultadoAcceso, verificar_acceso};
use limen_dominio::auditoria::CambioCampo;
use limen_dominio::contratista::{
    Contratista, ContratistaId, DatosContratista, ErrorContratista, HechosContratista,
    cedula_de_contratista,
};
use limen_dominio::empresa::EmpresaId;
use limen_dominio::tipo_ingreso::TipoIngreso;

use crate::errores::ErrorCaso;
use crate::puertos::{
    AccionAuditada, ConsultaPresencias, EntidadAuditada, EntradaAuditoria, FabricaUnidadDeTrabajo,
    GeneradorIds, RegistroAuditoria, Reloj, RepositorioContratistas, RepositorioEmpresas,
    Restriccion, UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorContratistas = ErrorCaso<ErrorContratista>;

/// La base rechazó la cédula porque otro equipo la guardó primero.
fn conflicto_de_cedula(restriccion: Restriccion) -> Option<ErrorContratista> {
    (restriccion == Restriccion::CedulaContratista).then_some(ErrorContratista::CedulaRepetida)
}

/// Lo que llega del formulario, tal cual se escribió.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComandoContratista {
    pub cedula: String,
    pub nombre: String,
    pub empresa: EmpresaId,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: NaiveDate,
    pub tiene_acceso: bool,
}

impl ComandoContratista {
    fn datos(&self) -> DatosContratista<'_> {
        DatosContratista {
            cedula: &self.cedula,
            nombre: &self.nombre,
            empresa: self.empresa,
            tipo_ingreso: self.tipo_ingreso,
            fecha_vencimiento_praind: self.fecha_vencimiento_praind,
            tiene_acceso: self.tiene_acceso,
        }
    }
}

#[derive(Debug)]
pub struct RegistrarContratista<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarContratista<F, R, G> {
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
        comando: &ComandoContratista,
    ) -> Result<ContratistaId, ErrorContratistas> {
        let cedula = cedula_de_contratista(&comando.cedula).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let hechos = HechosContratista {
            cedula_en_uso: uow.contratistas().cedula_en_uso(&cedula, None).await?,
            empresa_existe: uow.empresas().existe(comando.empresa).await?,
            esta_adentro: false,
        };
        let id = ContratistaId::desde_uuid(self.ids.nuevo());
        let contratista = Contratista::registrar(id, comando.datos(), hechos, self.reloj.hoy())
            .map_err(ErrorCaso::Negocio)?;

        uow.contratistas().guardar(&contratista);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            EntidadAuditada::Contratista,
            id.uuid(),
            AccionAuditada::Alta,
            contratista.cambios_de_alta(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_cedula))?;
        Ok(id)
    }
}

#[derive(Debug)]
pub struct EditarContratista<F, R> {
    fabrica: F,
    reloj: R,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj> EditarContratista<F, R> {
    pub const fn new(fabrica: F, reloj: R) -> Self {
        Self { fabrica, reloj }
    }

    /// Devuelve lo que cambió (vacío si no cambió nada: entonces no se
    /// escribe nada).
    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        id: ContratistaId,
        comando: &ComandoContratista,
    ) -> Result<Vec<CambioCampo>, ErrorContratistas> {
        let mut uow = self.fabrica.nueva();
        let mut contratista = uow
            .contratistas()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let cedula = cedula_de_contratista(&comando.cedula).map_err(ErrorCaso::Negocio)?;
        let hechos = HechosContratista {
            cedula_en_uso: uow.contratistas().cedula_en_uso(&cedula, Some(id)).await?,
            empresa_existe: uow.empresas().existe(comando.empresa).await?,
            esta_adentro: uow.presencias().esta_adentro(id).await?,
        };
        let cambios = contratista
            .editar(comando.datos(), hechos, self.reloj.hoy())
            .map_err(ErrorCaso::Negocio)?;
        if cambios.is_empty() {
            return Ok(cambios);
        }

        uow.contratistas().guardar(&contratista);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            EntidadAuditada::Contratista,
            id.uuid(),
            AccionAuditada::Edicion,
            cambios.clone(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_cedula))?;
        Ok(cambios)
    }
}

/// Un contratista y si puede entrar hoy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichaContratista {
    pub contratista: Contratista,
    pub acceso: ResultadoAcceso,
}

/// Busca un contratista por cédula (escrita en cualquier formato) y dice si
/// puede entrar hoy. Sólo lee: no confirma nada.
#[derive(Debug)]
pub struct ConsultarContratista<F, R> {
    fabrica: F,
    reloj: R,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj> ConsultarContratista<F, R> {
    pub const fn new(fabrica: F, reloj: R) -> Self {
        Self { fabrica, reloj }
    }

    pub async fn ejecutar(&self, cedula: &str) -> Result<FichaContratista, ErrorContratistas> {
        let cedula = cedula_de_contratista(cedula).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let contratista = uow
            .contratistas()
            .obtener_por_cedula(&cedula)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let acceso = verificar_acceso(&contratista, self.reloj.hoy());
        Ok(FichaContratista {
            contratista,
            acceso,
        })
    }
}
