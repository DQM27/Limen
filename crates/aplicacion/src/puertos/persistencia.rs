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

use chrono::{DateTime, Utc};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{Gafete, NumeroGafete, TipoGafete};
use limen_dominio::ingreso_contratista::{IngresoContratista, IngresoId};
use limen_dominio::ingreso_correo::{IngresoCorreo, IngresoCorreoId};
use limen_dominio::ingreso_proveedor::{IngresoProveedor, IngresoProveedorId};
use limen_dominio::personal_kof::{CodigoEmpleado, PersonalKof, PersonalKofId};
use limen_dominio::presencia::{Identidad, Via};
use limen_dominio::prestamo_kof::{PrestamoKof, PrestamoKofId};
use limen_dominio::usuario::{IntentosFallidos, Usuario};

use super::auditoria::RegistroAuditoria;
use super::hechos::RegistroHechos;
use crate::sesion::OperadorId;

/// Restricciones de unicidad que también hace cumplir la base. Si dos
/// equipos guardan lo mismo a la vez, la regla del dominio no alcanza a
/// verlo (cada uno leyó antes que el otro escribiera): la base rechaza al
/// segundo y el caso de uso lo traduce al error de negocio que corresponde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restriccion {
    CedulaContratista,
    NombreEmpresa,
    NombreEmpresaProveedora,
    /// Ya hay una persona del personal KOF con ese código de empleado.
    CodigoEmpleado,
    /// Esa persona del personal KOF ya tiene un provisional sin devolver.
    PersonalKofConPrestamo,
    /// Ya hay un gafete con ese tipo y número en el catálogo.
    NumeroGafete,
    /// La persona ya está adentro (por cualquier vía).
    PresenciaPersona,
    /// El gafete ya está prestado.
    GafetePrestado,
    /// Ya hay un usuario con esa cédula.
    CedulaUsuario,
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
    type Presencias: RepositorioPresencias;
    type Gafetes: RepositorioGafetes;
    type Ingresos: RepositorioIngresos;
    type EmpresasProveedoras: RepositorioEmpresasProveedoras;
    type IngresosProveedor: RepositorioIngresosProveedor;
    type IngresosCorreo: RepositorioIngresosCorreo;
    type PersonalKof: RepositorioPersonalKof;
    type PrestamosKof: RepositorioPrestamosKof;
    type Reloj: RepositorioReloj;
    type Usuarios: RepositorioUsuarios;
    type IntentosInicio: RepositorioIntentosInicio;
    type Auditoria: RegistroAuditoria;
    type Hechos: RegistroHechos;

    fn contratistas(&mut self) -> &mut Self::Contratistas;
    fn empresas(&mut self) -> &mut Self::Empresas;
    fn presencias(&mut self) -> &mut Self::Presencias;
    fn gafetes(&mut self) -> &mut Self::Gafetes;
    fn ingresos(&mut self) -> &mut Self::Ingresos;
    fn empresas_proveedoras(&mut self) -> &mut Self::EmpresasProveedoras;
    fn ingresos_proveedor(&mut self) -> &mut Self::IngresosProveedor;
    fn ingresos_correo(&mut self) -> &mut Self::IngresosCorreo;
    fn personal_kof(&mut self) -> &mut Self::PersonalKof;
    fn prestamos_kof(&mut self) -> &mut Self::PrestamosKof;
    fn reloj(&mut self) -> &mut Self::Reloj;
    fn usuarios(&mut self) -> &mut Self::Usuarios;
    fn intentos_inicio(&mut self) -> &mut Self::IntentosInicio;
    fn auditoria(&mut self) -> &mut Self::Auditoria;
    fn hechos(&mut self) -> &mut Self::Hechos;

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

/// Quién está adentro ahora y por qué vía, por cédula (reglas E1 y A7).
pub trait RepositorioPresencias: Send + Sync {
    fn via_adentro(
        &self,
        identidad: &Identidad,
    ) -> impl Future<Output = Result<Option<Via>, ErrorPersistencia>> + Send;

    /// Anota que la persona entró. Al confirmar choca
    /// ([`Restriccion::PresenciaPersona`]) si ya estaba adentro.
    fn anotar_entrada(&mut self, identidad: &Identidad, via: Via, desde: DateTime<Utc>);

    /// Anota que la persona salió.
    fn anotar_salida(&mut self, identidad: &Identidad);
}

/// Catálogo de gafetes y cuáles están prestados ahora.
pub trait RepositorioGafetes: Send + Sync {
    fn obtener(
        &self,
        tipo: TipoGafete,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<Gafete>, ErrorPersistencia>> + Send;

    /// Cuáles de `numeros` ya existen en el catálogo de `tipo`, de menor a
    /// mayor.
    fn existentes(
        &self,
        tipo: TipoGafete,
        numeros: &[NumeroGafete],
    ) -> impl Future<Output = Result<Vec<NumeroGafete>, ErrorPersistencia>> + Send;

    fn prestado(
        &self,
        tipo: TipoGafete,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Anota un gafete nuevo. Al confirmar choca
    /// ([`Restriccion::NumeroGafete`]) si el número ya existe en su tipo.
    fn agregar(&mut self, gafete: &Gafete);

    /// Anota el cambio de estado de un gafete existente.
    fn actualizar(&mut self, gafete: &Gafete);

    /// Anota que el gafete se prestó. Al confirmar choca
    /// ([`Restriccion::GafetePrestado`]) si ya estaba prestado.
    fn anotar_prestamo(&mut self, tipo: TipoGafete, numero: NumeroGafete, desde: DateTime<Utc>);

    /// Anota que el gafete se devolvió.
    fn anotar_devolucion(&mut self, tipo: TipoGafete, numero: NumeroGafete);
}

/// Ingresos de contratistas.
pub trait RepositorioIngresos: Send + Sync {
    fn obtener(
        &self,
        id: IngresoId,
    ) -> impl Future<Output = Result<Option<IngresoContratista>, ErrorPersistencia>> + Send;

    /// El ingreso abierto que tiene prestado el gafete de contratista
    /// `numero`, si hay alguno.
    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<IngresoContratista>, ErrorPersistencia>> + Send;

    /// Anota la entrada o la salida; se aplica al confirmar.
    fn guardar(&mut self, ingreso: &IngresoContratista);
}

/// La hora del último movimiento registrado en el equipo (regla E5).
pub trait RepositorioReloj: Send + Sync {
    fn ultimo_movimiento(
        &self,
    ) -> impl Future<Output = Result<Option<DateTime<Utc>>, ErrorPersistencia>> + Send;

    /// Anota la hora de un movimiento nuevo.
    fn anotar_movimiento(&mut self, en: DateTime<Utc>);
}

/// Catálogo de empresas proveedoras (aparte del de contratistas).
pub trait RepositorioEmpresasProveedoras: Send + Sync {
    fn obtener(
        &self,
        id: EmpresaProveedoraId,
    ) -> impl Future<Output = Result<Option<EmpresaProveedora>, ErrorPersistencia>> + Send;

    fn existe(
        &self,
        id: EmpresaProveedoraId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Si otra empresa proveedora (distinta de `excepto`) usa el nombre.
    fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaProveedoraId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Anota el alta o el cambio. Al confirmar falla con
    /// ([`Restriccion::NombreEmpresaProveedora`]) si el nombre ya está en uso.
    fn guardar(&mut self, empresa: &EmpresaProveedora);
}

pub trait RepositorioIngresosProveedor: Send + Sync {
    fn obtener(
        &self,
        id: IngresoProveedorId,
    ) -> impl Future<Output = Result<Option<IngresoProveedor>, ErrorPersistencia>> + Send;

    /// El ingreso de proveedor abierto que tiene prestado ese gafete de
    /// proveedor, si lo hay.
    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<IngresoProveedor>, ErrorPersistencia>> + Send;

    /// Anota el ingreso nuevo o su salida.
    fn guardar(&mut self, ingreso: &IngresoProveedor);
}

pub trait RepositorioIngresosCorreo: Send + Sync {
    fn obtener(
        &self,
        id: IngresoCorreoId,
    ) -> impl Future<Output = Result<Option<IngresoCorreo>, ErrorPersistencia>> + Send;

    /// El ingreso por correo abierto que tiene prestado ese gafete de
    /// visita, si lo hay.
    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<IngresoCorreo>, ErrorPersistencia>> + Send;

    /// Anota el ingreso nuevo o su salida.
    fn guardar(&mut self, ingreso: &IngresoCorreo);
}

/// Catálogo del personal KOF.
pub trait RepositorioPersonalKof: Send + Sync {
    fn obtener(
        &self,
        id: PersonalKofId,
    ) -> impl Future<Output = Result<Option<PersonalKof>, ErrorPersistencia>> + Send;

    /// Si otra persona (distinta de `excepto`) usa el código de empleado.
    fn codigo_en_uso(
        &self,
        codigo: &CodigoEmpleado,
        excepto: Option<PersonalKofId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Anota el alta o el cambio. Al confirmar falla con
    /// ([`Restriccion::CodigoEmpleado`]) si el código ya está en uso.
    fn guardar(&mut self, persona: &PersonalKof);
}

/// Préstamos de gafete provisional al personal KOF.
pub trait RepositorioPrestamosKof: Send + Sync {
    fn obtener(
        &self,
        id: PrestamoKofId,
    ) -> impl Future<Output = Result<Option<PrestamoKof>, ErrorPersistencia>> + Send;

    /// Si la persona tiene un provisional sin devolver.
    fn tiene_abierto(
        &self,
        personal: PersonalKofId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// El préstamo sin devolver de ese gafete provisional, si lo hay.
    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<PrestamoKof>, ErrorPersistencia>> + Send;

    /// Anota un préstamo nuevo. Al confirmar falla con
    /// ([`Restriccion::PersonalKofConPrestamo`]) si la persona ya tenía otro
    /// sin devolver.
    fn anotar_entrega(&mut self, prestamo: &PrestamoKof);

    /// Anota la devolución de un préstamo.
    fn anotar_devolucion(&mut self, prestamo: &PrestamoKof);
}

/// Usuarios del sistema (bloque L).
pub trait RepositorioUsuarios: Send + Sync {
    fn obtener(
        &self,
        id: OperadorId,
    ) -> impl Future<Output = Result<Option<Usuario>, ErrorPersistencia>> + Send;

    fn obtener_por_cedula(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<Option<Usuario>, ErrorPersistencia>> + Send;

    /// Si algún usuario tiene la cédula (la cédula de un usuario no se
    /// edita, así que no hace falta excluir a nadie).
    fn cedula_en_uso(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Si hay al menos un usuario (activo o no).
    fn hay_usuarios(&self) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send;

    /// Todos los usuarios, por nombre.
    fn todos(&self) -> impl Future<Output = Result<Vec<Usuario>, ErrorPersistencia>> + Send;

    /// Anota el alta o el cambio. Al confirmar falla con
    /// ([`Restriccion::CedulaUsuario`]) si la cédula ya está en uso.
    fn guardar(&mut self, usuario: &Usuario);
}

/// Intentos fallidos de inicio de sesión por cédula (regla L6). Son del
/// equipo: no se sincronizan.
pub trait RepositorioIntentosInicio: Send + Sync {
    fn obtener(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<Option<IntentosFallidos>, ErrorPersistencia>> + Send;

    /// Anota la nueva cuenta de intentos de la cédula.
    fn anotar(&mut self, cedula: &Cedula, intentos: IntentosFallidos);

    /// Anota que la cédula ya no tiene intentos fallidos.
    fn borrar(&mut self, cedula: &Cedula);
}
