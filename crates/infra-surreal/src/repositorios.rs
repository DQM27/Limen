//! Repositorios: lecturas inmediatas contra la base y escrituras anotadas
//! para la Unit of Work.

use limen_aplicacion::puertos::{
    ConsultaPresencias, EntradaAuditoria, ErrorPersistencia, RegistroAuditoria,
    RepositorioContratistas, RepositorioEmpresas,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;

use crate::error::tecnica;
use crate::registros::{
    AuditoriaRegistro, ContratistaDatos, ContratistaLeido, EmpresaDatos, EmpresaLeida,
    TABLA_AUDITORIA, TABLA_PRESENCIA, id_contratista, id_empresa, id_registro,
};

// --- Contratistas ---

#[derive(Debug)]
pub struct ContratistasSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<(RecordId, ContratistaDatos)>,
}

impl ContratistasSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioContratistas for ContratistasSurreal {
    async fn obtener(&self, id: ContratistaId) -> Result<Option<Contratista>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM ONLY $id")
            .bind(("id", id_contratista(id)))
            .await
            .map_err(tecnica)?;
        let leido: Option<ContratistaLeido> = respuesta.take(0).map_err(tecnica)?;
        leido.map(Contratista::try_from).transpose()
    }

    async fn obtener_por_cedula(
        &self,
        cedula: &Cedula,
    ) -> Result<Option<Contratista>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM contratista WHERE cedula = $cedula LIMIT 1")
            .bind(("cedula", cedula.as_str().to_owned()))
            .await
            .map_err(tecnica)?;
        let leidos: Vec<ContratistaLeido> = respuesta.take(0).map_err(tecnica)?;
        leidos
            .into_iter()
            .next()
            .map(Contratista::try_from)
            .transpose()
    }

    async fn cedula_en_uso(
        &self,
        cedula: &Cedula,
        excepto: Option<ContratistaId>,
    ) -> Result<bool, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query(
                "SELECT VALUE id FROM contratista WHERE cedula = $cedula AND id != $excepto LIMIT 1",
            )
            .bind(("cedula", cedula.as_str().to_owned()))
            .bind(("excepto", excepto.map(id_contratista)))
            .await
            .map_err(tecnica)?;
        let encontrados: Vec<RecordId> = respuesta.take(0).map_err(tecnica)?;
        Ok(!encontrados.is_empty())
    }

    fn guardar(&mut self, contratista: &Contratista) {
        self.pendientes.push((
            id_contratista(contratista.id()),
            ContratistaDatos::from(contratista),
        ));
    }
}

// --- Empresas ---

#[derive(Debug)]
pub struct EmpresasSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<(RecordId, EmpresaDatos)>,
}

impl EmpresasSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioEmpresas for EmpresasSurreal {
    async fn obtener(&self, id: EmpresaId) -> Result<Option<Empresa>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM ONLY $id")
            .bind(("id", id_empresa(id)))
            .await
            .map_err(tecnica)?;
        let leida: Option<EmpresaLeida> = respuesta.take(0).map_err(tecnica)?;
        leida.map(Empresa::try_from).transpose()
    }

    async fn existe(&self, id: EmpresaId) -> Result<bool, ErrorPersistencia> {
        existe_registro(&self.db, id_empresa(id)).await
    }

    async fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaId>,
    ) -> Result<bool, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT VALUE id FROM empresa WHERE nombre = $nombre AND id != $excepto LIMIT 1")
            .bind(("nombre", nombre.as_str().to_owned()))
            .bind(("excepto", excepto.map(id_empresa)))
            .await
            .map_err(tecnica)?;
        let encontradas: Vec<RecordId> = respuesta.take(0).map_err(tecnica)?;
        Ok(!encontradas.is_empty())
    }

    fn guardar(&mut self, empresa: &Empresa) {
        self.pendientes
            .push((id_empresa(empresa.id()), EmpresaDatos::from(empresa)));
    }
}

// --- Presencias ---

#[derive(Debug)]
pub struct PresenciasSurreal {
    db: Surreal<Db>,
}

impl PresenciasSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self { db }
    }
}

impl ConsultaPresencias for PresenciasSurreal {
    async fn esta_adentro(&self, contratista: ContratistaId) -> Result<bool, ErrorPersistencia> {
        existe_registro(&self.db, id_registro(TABLA_PRESENCIA, contratista.uuid())).await
    }
}

// --- Auditoría ---

#[derive(Debug, Default)]
pub struct AuditoriaSurreal {
    pub(crate) pendientes: Vec<(RecordId, AuditoriaRegistro)>,
}

impl RegistroAuditoria for AuditoriaSurreal {
    fn anotar(&mut self, entrada: EntradaAuditoria) {
        self.pendientes.push((
            id_registro(TABLA_AUDITORIA, entrada.id_entrada),
            AuditoriaRegistro::from(&entrada),
        ));
    }
}

/// Si existe el registro con ese ID.
async fn existe_registro(db: &Surreal<Db>, id: RecordId) -> Result<bool, ErrorPersistencia> {
    let mut respuesta = db
        .query("SELECT VALUE id FROM ONLY $id")
        .bind(("id", id))
        .await
        .map_err(tecnica)?;
    let encontrado: Option<RecordId> = respuesta.take(0).map_err(tecnica)?;
    Ok(encontrado.is_some())
}
