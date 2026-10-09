//! Repositorios: lecturas inmediatas contra la base y escrituras anotadas
//! para la Unit of Work.

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{
    EntradaAuditoria, ErrorPersistencia, RegistroAuditoria, RepositorioContratistas,
    RepositorioEmpresas, RepositorioEmpresasProveedoras, RepositorioGafetes, RepositorioIngresos,
    RepositorioIngresosCorreo, RepositorioIngresosProveedor, RepositorioPresencias,
    RepositorioReloj,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{Gafete, NumeroGafete, TipoGafete};
use limen_dominio::ingreso_contratista::{IngresoContratista, IngresoId};
use limen_dominio::ingreso_correo::{IngresoCorreo, IngresoCorreoId};
use limen_dominio::ingreso_proveedor::{IngresoProveedor, IngresoProveedorId};
use limen_dominio::presencia::Via;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{RecordId, SurrealValue, Value};

use crate::error::{dato_corrupto, tecnica};
use crate::registros::{
    AuditoriaRegistro, ContratistaDatos, ContratistaLeido, EmpresaDatos, EmpresaLeida,
    EmpresaProveedoraLeida, GafeteDatos, IngresoCorreoDatos, IngresoCorreoLeido, IngresoDatos,
    IngresoLeido, IngresoProveedorDatos, IngresoProveedorLeido, PresenciaDatos, PrestamoDatos,
    RelojDatos, TABLA_AUDITORIA, TABLA_GAFETE, TABLA_PRESENCIA, id_contratista, id_empresa,
    id_empresa_proveedora, id_gafete, id_ingreso, id_ingreso_correo, id_ingreso_proveedor,
    id_presencia, id_prestamo, id_registro, id_reloj,
};

/// Una escritura anotada, pendiente de confirmar.
#[derive(Debug, Clone)]
pub enum Escritura {
    /// Crea el registro; falla si ya existe. Es lo que hace cumplir las
    /// claves naturales (un gafete, un préstamo, una presencia).
    Crear(RecordId, Value),
    /// Crea o reemplaza el registro.
    Guardar(RecordId, Value),
    /// Borra el registro si existe.
    Borrar(RecordId),
}

impl Escritura {
    fn crear(id: RecordId, datos: impl SurrealValue) -> Self {
        Self::Crear(id, datos.into_value())
    }

    fn guardar(id: RecordId, datos: impl SurrealValue) -> Self {
        Self::Guardar(id, datos.into_value())
    }
}

// --- Contratistas ---

#[derive(Debug)]
pub struct ContratistasSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
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
        self.pendientes.push(Escritura::guardar(
            id_contratista(contratista.id()),
            ContratistaDatos::from(contratista),
        ));
    }
}

// --- Empresas ---

#[derive(Debug)]
pub struct EmpresasSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
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
        self.pendientes.push(Escritura::guardar(
            id_empresa(empresa.id()),
            EmpresaDatos::from(empresa),
        ));
    }
}

// --- Presencias ---

#[derive(Debug)]
pub struct PresenciasSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
}

impl PresenciasSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioPresencias for PresenciasSurreal {
    async fn via_adentro(&self, cedula: &Cedula) -> Result<Option<Via>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT VALUE via FROM ONLY $id")
            .bind(("id", id_presencia(cedula)))
            .await
            .map_err(tecnica)?;
        let via: Option<String> = respuesta.take(0).map_err(tecnica)?;
        via.map(|codigo| {
            Via::desde_codigo(&codigo)
                .ok_or_else(|| dato_corrupto(TABLA_PRESENCIA, format!("vía desconocida: {codigo}")))
        })
        .transpose()
    }

    /// Crea `presencia:⟨cédula⟩`: si la persona ya está adentro, la base
    /// rechaza la transacción.
    fn anotar_entrada(&mut self, cedula: &Cedula, via: Via, desde: DateTime<Utc>) {
        self.pendientes.push(Escritura::crear(
            id_presencia(cedula),
            PresenciaDatos {
                via: via.codigo().to_owned(),
                desde,
            },
        ));
    }

    fn anotar_salida(&mut self, cedula: &Cedula) {
        self.pendientes
            .push(Escritura::Borrar(id_presencia(cedula)));
    }
}

// --- Gafetes ---

#[derive(Debug)]
pub struct GafetesSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
}

impl GafetesSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioGafetes for GafetesSurreal {
    async fn obtener(
        &self,
        tipo: TipoGafete,
        numero: NumeroGafete,
    ) -> Result<Option<Gafete>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * OMIT id FROM ONLY $id")
            .bind(("id", id_gafete(tipo, numero)))
            .await
            .map_err(tecnica)?;
        let datos: Option<GafeteDatos> = respuesta.take(0).map_err(tecnica)?;
        datos.map(Gafete::try_from).transpose()
    }

    async fn existentes(
        &self,
        tipo: TipoGafete,
        numeros: &[NumeroGafete],
    ) -> Result<Vec<NumeroGafete>, ErrorPersistencia> {
        if numeros.is_empty() {
            return Ok(Vec::new());
        }
        let buscados: Vec<i64> = numeros.iter().map(|n| i64::from(n.valor())).collect();
        let mut respuesta = self
            .db
            .query(
                "SELECT VALUE numero FROM gafete WHERE tipo = $tipo AND numero IN $numeros \
                 ORDER BY numero",
            )
            .bind(("tipo", tipo.codigo().to_owned()))
            .bind(("numeros", buscados))
            .await
            .map_err(tecnica)?;
        let encontrados: Vec<i64> = respuesta.take(0).map_err(tecnica)?;
        encontrados
            .into_iter()
            .map(|valor| {
                u32::try_from(valor)
                    .ok()
                    .and_then(|n| NumeroGafete::nuevo(n).ok())
                    .ok_or_else(|| dato_corrupto(TABLA_GAFETE, format!("número inválido: {valor}")))
            })
            .collect()
    }

    async fn prestado(
        &self,
        tipo: TipoGafete,
        numero: NumeroGafete,
    ) -> Result<bool, ErrorPersistencia> {
        existe_registro(&self.db, id_prestamo(tipo, numero)).await
    }

    /// Crea `gafete:⟨TIPO-NÚMERO⟩`: un número repetido hace fallar la
    /// transacción.
    fn agregar(&mut self, gafete: &Gafete) {
        self.pendientes.push(Escritura::crear(
            id_gafete(gafete.tipo(), gafete.numero()),
            GafeteDatos::from(gafete),
        ));
    }

    fn actualizar(&mut self, gafete: &Gafete) {
        self.pendientes.push(Escritura::guardar(
            id_gafete(gafete.tipo(), gafete.numero()),
            GafeteDatos::from(gafete),
        ));
    }

    /// Crea `prestamo_gafete:⟨TIPO-NÚMERO⟩`: si otro equipo ya lo prestó,
    /// la transacción falla.
    fn anotar_prestamo(&mut self, tipo: TipoGafete, numero: NumeroGafete, desde: DateTime<Utc>) {
        self.pendientes.push(Escritura::crear(
            id_prestamo(tipo, numero),
            PrestamoDatos { desde },
        ));
    }

    fn anotar_devolucion(&mut self, tipo: TipoGafete, numero: NumeroGafete) {
        self.pendientes
            .push(Escritura::Borrar(id_prestamo(tipo, numero)));
    }
}

// --- Ingresos ---

#[derive(Debug)]
pub struct IngresosSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
}

impl IngresosSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioIngresos for IngresosSurreal {
    async fn obtener(
        &self,
        id: IngresoId,
    ) -> Result<Option<IngresoContratista>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM ONLY $id")
            .bind(("id", id_ingreso(id)))
            .await
            .map_err(tecnica)?;
        let leido: Option<IngresoLeido> = respuesta.take(0).map_err(tecnica)?;
        leido.map(IngresoContratista::try_from).transpose()
    }

    async fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> Result<Option<IngresoContratista>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query(
                "SELECT * FROM ingreso_contratista \
                 WHERE gafete = $gafete AND salida_en = NONE LIMIT 1",
            )
            .bind(("gafete", i64::from(numero.valor())))
            .await
            .map_err(tecnica)?;
        let leidos: Vec<IngresoLeido> = respuesta.take(0).map_err(tecnica)?;
        leidos
            .into_iter()
            .next()
            .map(IngresoContratista::try_from)
            .transpose()
    }

    fn guardar(&mut self, ingreso: &IngresoContratista) {
        self.pendientes.push(Escritura::guardar(
            id_ingreso(ingreso.id()),
            IngresoDatos::from(ingreso),
        ));
    }
}

// --- Empresas proveedoras ---

#[derive(Debug)]
pub struct EmpresasProveedorasSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
}

impl EmpresasProveedorasSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioEmpresasProveedoras for EmpresasProveedorasSurreal {
    async fn obtener(
        &self,
        id: EmpresaProveedoraId,
    ) -> Result<Option<EmpresaProveedora>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM ONLY $id")
            .bind(("id", id_empresa_proveedora(id)))
            .await
            .map_err(tecnica)?;
        let leida: Option<EmpresaProveedoraLeida> = respuesta.take(0).map_err(tecnica)?;
        leida.map(EmpresaProveedora::try_from).transpose()
    }

    async fn existe(&self, id: EmpresaProveedoraId) -> Result<bool, ErrorPersistencia> {
        existe_registro(&self.db, id_empresa_proveedora(id)).await
    }

    async fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaProveedoraId>,
    ) -> Result<bool, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query(
                "SELECT VALUE id FROM empresa_proveedora \
                 WHERE nombre = $nombre AND id != $excepto LIMIT 1",
            )
            .bind(("nombre", nombre.as_str().to_owned()))
            .bind(("excepto", excepto.map(id_empresa_proveedora)))
            .await
            .map_err(tecnica)?;
        let encontradas: Vec<RecordId> = respuesta.take(0).map_err(tecnica)?;
        Ok(!encontradas.is_empty())
    }

    fn guardar(&mut self, empresa: &EmpresaProveedora) {
        self.pendientes.push(Escritura::guardar(
            id_empresa_proveedora(empresa.id()),
            EmpresaDatos::from(empresa),
        ));
    }
}

// --- Ingresos de proveedores ---

#[derive(Debug)]
pub struct IngresosProveedorSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
}

impl IngresosProveedorSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioIngresosProveedor for IngresosProveedorSurreal {
    async fn obtener(
        &self,
        id: IngresoProveedorId,
    ) -> Result<Option<IngresoProveedor>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM ONLY $id")
            .bind(("id", id_ingreso_proveedor(id)))
            .await
            .map_err(tecnica)?;
        let leido: Option<IngresoProveedorLeido> = respuesta.take(0).map_err(tecnica)?;
        leido.map(IngresoProveedor::try_from).transpose()
    }

    async fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> Result<Option<IngresoProveedor>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query(
                "SELECT * FROM ingreso_proveedor \
                 WHERE gafete = $gafete AND salida_en = NONE LIMIT 1",
            )
            .bind(("gafete", i64::from(numero.valor())))
            .await
            .map_err(tecnica)?;
        let leidos: Vec<IngresoProveedorLeido> = respuesta.take(0).map_err(tecnica)?;
        leidos
            .into_iter()
            .next()
            .map(IngresoProveedor::try_from)
            .transpose()
    }

    fn guardar(&mut self, ingreso: &IngresoProveedor) {
        self.pendientes.push(Escritura::guardar(
            id_ingreso_proveedor(ingreso.id()),
            IngresoProveedorDatos::from(ingreso),
        ));
    }
}

// --- Ingresos por correo ---

#[derive(Debug)]
pub struct IngresosCorreoSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
}

impl IngresosCorreoSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioIngresosCorreo for IngresosCorreoSurreal {
    async fn obtener(
        &self,
        id: IngresoCorreoId,
    ) -> Result<Option<IngresoCorreo>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM ONLY $id")
            .bind(("id", id_ingreso_correo(id)))
            .await
            .map_err(tecnica)?;
        let leido: Option<IngresoCorreoLeido> = respuesta.take(0).map_err(tecnica)?;
        leido.map(IngresoCorreo::try_from).transpose()
    }

    async fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> Result<Option<IngresoCorreo>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query(
                "SELECT * FROM ingreso_correo \
                 WHERE gafete = $gafete AND salida_en = NONE LIMIT 1",
            )
            .bind(("gafete", i64::from(numero.valor())))
            .await
            .map_err(tecnica)?;
        let leidos: Vec<IngresoCorreoLeido> = respuesta.take(0).map_err(tecnica)?;
        leidos
            .into_iter()
            .next()
            .map(IngresoCorreo::try_from)
            .transpose()
    }

    fn guardar(&mut self, ingreso: &IngresoCorreo) {
        self.pendientes.push(Escritura::guardar(
            id_ingreso_correo(ingreso.id()),
            IngresoCorreoDatos::from(ingreso),
        ));
    }
}

// --- Reloj ---

#[derive(Debug)]
pub struct RelojSurreal {
    db: Surreal<Db>,
    pub(crate) pendientes: Vec<Escritura>,
}

impl RelojSurreal {
    pub(crate) const fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            pendientes: Vec::new(),
        }
    }
}

impl RepositorioReloj for RelojSurreal {
    async fn ultimo_movimiento(&self) -> Result<Option<DateTime<Utc>>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT VALUE en FROM ONLY $id")
            .bind(("id", id_reloj()))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    fn anotar_movimiento(&mut self, en: DateTime<Utc>) {
        self.pendientes
            .push(Escritura::guardar(id_reloj(), RelojDatos { en }));
    }
}

// --- Auditoría ---

#[derive(Debug, Default)]
pub struct AuditoriaSurreal {
    pub(crate) pendientes: Vec<Escritura>,
}

impl RegistroAuditoria for AuditoriaSurreal {
    fn anotar(&mut self, entrada: EntradaAuditoria) {
        self.pendientes.push(Escritura::crear(
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
