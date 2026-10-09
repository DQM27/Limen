//! Persistencia: Unit of Work y repositorios.
//!
//! La Unit of Work sigue el patrón de Martin Fowler: las **lecturas** van a
//! la base en el momento; las **escrituras** sólo se anotan y se aplican
//! todas juntas, en una sola transacción, al llamar a
//! [`UnidadDeTrabajo::confirmar`]. Si no se confirma, no se escribe nada.
//!
//! Consecuencia: dentro de una misma Unit of Work, una lectura no ve las
//! escrituras anotadas antes. Los casos de uso leen primero y escriben al
//! final.

use std::future::Future;

use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};

use super::auditoria::RegistroAuditoria;

/// Restricciones de unicidad que también hace cumplir la base. Si dos
/// equipos guardan lo mismo a la vez, la regla del dominio no alcanza a
/// verlo (cada uno leyó antes que el otro escribiera): la base rechaza al
/// segundo y el caso de uso lo traduce al error de negocio que corresponde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restriccion {
    CedulaContratista,
    NombreEmpresa,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorPersistencia {
    #[error("otro registro ya ocupa la restricción {0:?}")]
    Conflicto(Restriccion),
    /// Cualquier otra falla (base dañada, disco lleno...). El texto es para
    /// el registro técnico, nunca para el operador.
    #[error("falla técnica de persistencia: {0}")]
    Tecnica(String),
}

/// Crea una Unit of Work nueva por cada operación.
pub trait FabricaUnidadDeTrabajo: Send + Sync {
    type Uow: UnidadDeTrabajo;

    fn nueva(&self) -> Self::Uow;
}

pub trait UnidadDeTrabajo: Send {
    type Contratistas: RepositorioContratistas;
    type Empresas: RepositorioEmpresas;
    type Presencias: ConsultaPresencias;
    type Auditoria: RegistroAuditoria;

    fn contratistas(&mut self) -> &mut Self::Contratistas;
    fn empresas(&mut self) -> &mut Self::Empresas;
    fn presencias(&self) -> &Self::Presencias;
    fn auditoria(&mut self) -> &mut Self::Auditoria;

    /// Aplica todas las escrituras anotadas en una sola transacción: todas o
    /// ninguna.
    fn confirmar(self) -> impl Future<Output = Result<(), ErrorPersistencia>> + Send;
}

pub trait RepositorioContratistas: Send + Sync {
    fn obtener(
        &self,
        id: ContratistaId,
    ) -> impl Future<Output = Result<Option<Contratista>, ErrorPersistencia>> + Send;

    fn obtener_por_cedula(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<Option<Contratista>, ErrorPersistencia>> + Send;

    /// Si un contratista distinto de `excepto` ya tiene la cédula.
    fn cedula_en_uso(
        &self,
        cedula: &Cedula,
        excepto: Option<ContratistaId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Anota el alta o el cambio; se aplica al confirmar.
    fn guardar(&mut self, contratista: &Contratista);
}

pub trait RepositorioEmpresas: Send + Sync {
    fn obtener(
        &self,
        id: EmpresaId,
    ) -> impl Future<Output = Result<Option<Empresa>, ErrorPersistencia>> + Send;

    fn existe(&self, id: EmpresaId)
    -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Si una empresa distinta de `excepto` ya usa el nombre.
    fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Anota el alta o el cambio; se aplica al confirmar.
    fn guardar(&mut self, empresa: &Empresa);
}

/// Quién está adentro ahora mismo. La llena el módulo de ingresos (todavía
/// no existe); los contratistas sólo la consultan.
pub trait ConsultaPresencias: Send + Sync {
    fn esta_adentro(
        &self,
        contratista: ContratistaId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;
}
