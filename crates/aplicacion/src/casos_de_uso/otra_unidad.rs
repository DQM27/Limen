//! Regla A9: antes de registrar una entrada, ver si la persona ya está
//! adentro en otra unidad.
//!
//! Va antes de cada una de las cuatro vías de entrada. El caso de uso sólo
//! averigua quién es la persona y le pregunta a la nube; quien decide si se
//! puede seguir es el dominio (`verificar_no_adentro_en_otra_unidad`).

use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::ContratistaId;
use limen_dominio::origen::UnidadOperativaId;
use limen_dominio::personal_kof::PersonalKofId;
use limen_dominio::presencia::{
    AdentroEnOtraUnidad, Identidad, verificar_no_adentro_en_otra_unidad,
};

use crate::errores::ErrorCaso;
use crate::puertos::{
    FabricaUnidadDeTrabajo, PresenciaRemota, RepositorioContratistas, RepositorioPersonalKof,
    UnidadDeTrabajo,
};

pub type ErrorOtraUnidad = ErrorCaso<AdentroEnOtraUnidad>;

#[derive(Debug)]
pub struct VerificarOtraUnidad<F, P> {
    fabrica: F,
    remota: P,
    /// La unidad de este equipo: lo que la nube reporte de ella no cuenta
    /// (eso lo cubre la presencia local, regla E1).
    unidad: UnidadOperativaId,
}

impl<F: FabricaUnidadDeTrabajo, P: PresenciaRemota> VerificarOtraUnidad<F, P> {
    pub const fn new(fabrica: F, remota: P, unidad: UnidadOperativaId) -> Self {
        Self {
            fabrica,
            remota,
            unidad,
        }
    }

    /// Antes de registrar la entrada de un contratista. Si el contratista no
    /// existe se deja pasar: la entrada misma responde `NoEncontrado`.
    pub async fn contratista(&self, id: ContratistaId) -> Result<(), ErrorOtraUnidad> {
        let mut uow = self.fabrica.nueva();
        let Some(contratista) = uow.contratistas().obtener(id).await? else {
            return Ok(());
        };
        self.verificar(&Identidad::from(contratista.cedula())).await
    }

    /// Antes de registrar la entrada de un proveedor o de un ingreso por
    /// correo, que llegan con la cédula escrita. Una cédula mal escrita se
    /// deja pasar: la validación de la entrada ya se lo dice al operador.
    pub async fn cedula(&self, texto: &str) -> Result<(), ErrorOtraUnidad> {
        match Cedula::normalizar(texto) {
            Ok(cedula) => self.verificar(&Identidad::from(&cedula)).await,
            Err(_) => Ok(()),
        }
    }

    /// Antes de entregar el gafete provisional al personal KOF (su entrada).
    pub async fn personal_kof(&self, id: PersonalKofId) -> Result<(), ErrorOtraUnidad> {
        let mut uow = self.fabrica.nueva();
        let Some(persona) = uow.personal_kof().obtener(id).await? else {
            return Ok(());
        };
        self.verificar(&Identidad::from(persona.codigo())).await
    }

    async fn verificar(&self, identidad: &Identidad) -> Result<(), ErrorOtraUnidad> {
        let reportado = self.remota.consultar(identidad).await;
        verificar_no_adentro_en_otra_unidad(reportado, &self.unidad).map_err(ErrorCaso::Negocio)
    }
}
