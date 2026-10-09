//! Consultas de lectura: lo que las pantallas necesitan mostrar y que no
//! forma parte de ninguna escritura.
//!
//! Son un puerto aparte de la Unit of Work porque sólo leen: no anotan
//! nada ni se confirman. Cada método devuelve datos ya armados para la
//! pantalla (por ejemplo, el nombre de la empresa junto al contratista).

use std::future::Future;

use chrono::{DateTime, NaiveDate, Utc};
use limen_dominio::busqueda::Criterio;
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::Empresa;
use limen_dominio::empresa_proveedora::EmpresaProveedora;
use limen_dominio::gafete::{Gafete, NumeroGafete, TipoGafete};
use limen_dominio::ingreso_contratista::IngresoId;
use limen_dominio::ingreso_correo::IngresoCorreoId;
use limen_dominio::ingreso_proveedor::IngresoProveedorId;
use limen_dominio::medio::Medio;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::personal_kof::PersonalKof;
use limen_dominio::presencia::Via;

use super::auditoria::{AccionAuditada, RegistroAuditado};
use super::persistencia::ErrorPersistencia;

/// El ingreso abierto de alguien que está adentro, según su vía. Es lo que
/// la pantalla necesita para registrar su salida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngresoAbierto {
    Contratista(IngresoId),
    Proveedor(IngresoProveedorId),
    Correo(IngresoCorreoId),
}

impl IngresoAbierto {
    pub const fn via(self) -> Via {
        match self {
            Self::Contratista(_) => Via::Contratista,
            Self::Proveedor(_) => Via::Proveedor,
            Self::Correo(_) => Via::Correo,
        }
    }
}

/// Una persona que está adentro ahora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaAdentro {
    pub ingreso: IngresoAbierto,
    pub cedula: Cedula,
    pub nombre: NombrePersona,
    /// De dónde viene: la empresa (contratista y proveedor) o el motivo de
    /// la visita (ingreso por correo).
    pub procedencia: String,
    pub medio: Medio,
    pub gafete: Option<NumeroGafete>,
    pub desde: DateTime<Utc>,
}

/// Un cambio guardado en el historial de un registro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioHistorial {
    pub campo: String,
    pub antes: String,
    pub despues: String,
}

/// Una entrada del historial de un registro: qué se hizo, quién y cuándo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntradaHistorial {
    pub accion: AccionAuditada,
    pub cambios: Vec<CambioHistorial>,
    pub operador: OperadorId,
    pub en: DateTime<Utc>,
}

/// Un gafete del catálogo y si está prestado ahora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumenGafete {
    pub gafete: Gafete,
    pub prestado: bool,
}

pub trait Consultas: Send + Sync {
    /// Quién está adentro, del ingreso más reciente al más antiguo (con la
    /// cédula como desempate).
    fn quienes_estan_adentro(
        &self,
    ) -> impl Future<Output = Result<Vec<PersonaAdentro>, ErrorPersistencia>> + Send;

    /// Contratistas que cumplen el criterio, del mejor al peor resultado
    /// (ver [`limen_dominio::busqueda`]), hasta `limite`.
    fn buscar_contratistas(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<Contratista>, ErrorPersistencia>> + Send;

    /// Personal KOF (activo o no) que cumple el criterio, del mejor al peor
    /// resultado, hasta `limite`.
    fn buscar_personal_kof(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<PersonalKof>, ErrorPersistencia>> + Send;

    /// Empresas de contratistas cuyo nombre cumple el criterio, del mejor al
    /// peor resultado, hasta `limite`.
    fn buscar_empresas(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<Empresa>, ErrorPersistencia>> + Send;

    /// Lo mismo, para las empresas proveedoras.
    fn buscar_empresas_proveedoras(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<EmpresaProveedora>, ErrorPersistencia>> + Send;

    /// El historial de cambios de un registro, del más antiguo al más
    /// reciente (regla B11: todo cambio queda, por insignificante que sea).
    fn historial_de(
        &self,
        registro: RegistroAuditado,
    ) -> impl Future<Output = Result<Vec<EntradaHistorial>, ErrorPersistencia>> + Send;

    /// Los gafetes de un tipo, por número, con su estado y si están
    /// prestados.
    fn listar_gafetes(
        &self,
        tipo: TipoGafete,
    ) -> impl Future<Output = Result<Vec<ResumenGafete>, ErrorPersistencia>> + Send;

    /// Contratistas con acceso cuyo PRAIND vence a más tardar en `hasta`
    /// (incluidos los ya vencidos), del que vence primero al último; a
    /// igual fecha, por nombre y cédula. Hasta `limite`.
    fn contratistas_con_praind_hasta(
        &self,
        hasta: NaiveDate,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<Contratista>, ErrorPersistencia>> + Send;
}
