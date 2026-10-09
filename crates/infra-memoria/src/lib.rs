//! Implementación en memoria de los puertos de Limen, para probar los casos
//! de uso sin base de datos.
//!
//! No es un juguete: se comporta como la base real en lo que importa a los
//! casos de uso, para que una prueba que pasa acá también pase contra
//! `SurrealDB` (eso lo verificarán las pruebas de contrato):
//!
//! - las lecturas ven sólo lo confirmado, nunca lo anotado en la misma
//!   Unit of Work;
//! - `confirmar()` aplica todo o nada;
//! - al confirmar se revisan las mismas restricciones de unicidad que la
//!   base (cédula de contratista, nombre de empresa).
//!
//! Además permite sembrar datos y simular fallas para las pruebas.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, NaiveDate, Utc};
use limen_aplicacion::puertos::{
    ConsultaPresencias, EntradaAuditoria, ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds,
    RegistroAuditoria, Reloj, RepositorioContratistas, RepositorioEmpresas, Restriccion,
    UnidadDeTrabajo,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use uuid::Uuid;

#[derive(Debug, Default)]
struct Datos {
    contratistas: BTreeMap<ContratistaId, Contratista>,
    empresas: BTreeMap<EmpresaId, Empresa>,
    adentro: BTreeSet<ContratistaId>,
    auditoria: Vec<EntradaAuditoria>,
    confirmaciones: usize,
    falla_proxima_confirmacion: Option<ErrorPersistencia>,
    falla_lecturas: Option<ErrorPersistencia>,
}

/// La "base de datos" en memoria. Clonarla comparte los mismos datos, como
/// dos conexiones a la misma base.
#[derive(Debug, Clone, Default)]
pub struct AlmacenMemoria {
    datos: Arc<Mutex<Datos>>,
}

impl AlmacenMemoria {
    pub fn new() -> Self {
        Self::default()
    }

    fn bloquear(&self) -> MutexGuard<'_, Datos> {
        // Un pánico en otra prueba no debe envenenar a ésta.
        self.datos.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn leer<T>(&self, lectura: impl FnOnce(&Datos) -> T) -> Result<T, ErrorPersistencia> {
        let datos = self.bloquear();
        datos
            .falla_lecturas
            .clone()
            .map_or_else(|| Ok(lectura(&datos)), Err)
    }

    // --- Para preparar y revisar las pruebas ---

    /// Guarda una empresa directo, como si ya existiera.
    pub fn sembrar_empresa(&self, empresa: Empresa) {
        self.bloquear().empresas.insert(empresa.id(), empresa);
    }

    /// Guarda un contratista directo, como si ya existiera.
    pub fn sembrar_contratista(&self, contratista: Contratista) {
        self.bloquear()
            .contratistas
            .insert(contratista.id(), contratista);
    }

    /// Marca a un contratista como adentro (lo hará el módulo de ingresos).
    pub fn marcar_adentro(&self, contratista: ContratistaId) {
        self.bloquear().adentro.insert(contratista);
    }

    /// La próxima confirmación falla con `error`, sin aplicar nada.
    pub fn fallar_proxima_confirmacion(&self, error: ErrorPersistencia) {
        self.bloquear().falla_proxima_confirmacion = Some(error);
    }

    /// Todas las lecturas fallan con `error` desde ahora.
    pub fn fallar_lecturas(&self, error: ErrorPersistencia) {
        self.bloquear().falla_lecturas = Some(error);
    }

    pub fn contratistas(&self) -> Vec<Contratista> {
        self.bloquear().contratistas.values().cloned().collect()
    }

    pub fn empresas(&self) -> Vec<Empresa> {
        self.bloquear().empresas.values().cloned().collect()
    }

    pub fn auditoria(&self) -> Vec<EntradaAuditoria> {
        self.bloquear().auditoria.clone()
    }

    /// Cuántas veces se confirmó con éxito una Unit of Work.
    pub fn confirmaciones(&self) -> usize {
        self.bloquear().confirmaciones
    }
}

impl FabricaUnidadDeTrabajo for AlmacenMemoria {
    type Uow = UowMemoria;

    fn nueva(&self) -> UowMemoria {
        UowMemoria {
            almacen: self.clone(),
            contratistas: ContratistasMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            empresas: EmpresasMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            presencias: PresenciasMemoria {
                almacen: self.clone(),
            },
            auditoria: AuditoriaMemoria {
                pendientes: Vec::new(),
            },
        }
    }
}

#[derive(Debug)]
pub struct UowMemoria {
    almacen: AlmacenMemoria,
    contratistas: ContratistasMemoria,
    empresas: EmpresasMemoria,
    presencias: PresenciasMemoria,
    auditoria: AuditoriaMemoria,
}

impl UnidadDeTrabajo for UowMemoria {
    type Contratistas = ContratistasMemoria;
    type Empresas = EmpresasMemoria;
    type Presencias = PresenciasMemoria;
    type Auditoria = AuditoriaMemoria;

    fn contratistas(&mut self) -> &mut ContratistasMemoria {
        &mut self.contratistas
    }

    fn empresas(&mut self) -> &mut EmpresasMemoria {
        &mut self.empresas
    }

    fn presencias(&self) -> &PresenciasMemoria {
        &self.presencias
    }

    fn auditoria(&mut self) -> &mut AuditoriaMemoria {
        &mut self.auditoria
    }

    async fn confirmar(self) -> Result<(), ErrorPersistencia> {
        let mut datos = self.almacen.bloquear();
        if let Some(error) = datos.falla_proxima_confirmacion.take() {
            return Err(error);
        }
        verificar_cedulas_unicas(&datos.contratistas, &self.contratistas.pendientes)?;
        verificar_nombres_unicos(&datos.empresas, &self.empresas.pendientes)?;

        // Todas las restricciones pasaron: se aplica todo junto.
        for contratista in self.contratistas.pendientes {
            datos.contratistas.insert(contratista.id(), contratista);
        }
        for empresa in self.empresas.pendientes {
            datos.empresas.insert(empresa.id(), empresa);
        }
        datos.auditoria.extend(self.auditoria.pendientes);
        datos.confirmaciones += 1;
        drop(datos);
        Ok(())
    }
}

/// Lo que hará el índice único de la base: ninguna cédula repetida entre
/// contratistas distintos, ni contra lo guardado ni dentro de lo pendiente.
fn verificar_cedulas_unicas(
    guardados: &BTreeMap<ContratistaId, Contratista>,
    pendientes: &[Contratista],
) -> Result<(), ErrorPersistencia> {
    let conflicto = ErrorPersistencia::Conflicto(Restriccion::CedulaContratista);
    let mut vistas: BTreeMap<&str, ContratistaId> = guardados
        .values()
        .filter(|guardado| pendientes.iter().all(|p| p.id() != guardado.id()))
        .map(|guardado| (guardado.cedula().as_str(), guardado.id()))
        .collect();
    for pendiente in pendientes {
        if let Some(otro) = vistas.insert(pendiente.cedula().as_str(), pendiente.id())
            && otro != pendiente.id()
        {
            return Err(conflicto);
        }
    }
    Ok(())
}

/// Lo que hará el índice único de la base para el nombre de empresa.
fn verificar_nombres_unicos(
    guardadas: &BTreeMap<EmpresaId, Empresa>,
    pendientes: &[Empresa],
) -> Result<(), ErrorPersistencia> {
    let conflicto = ErrorPersistencia::Conflicto(Restriccion::NombreEmpresa);
    let mut vistos: BTreeMap<&str, EmpresaId> = guardadas
        .values()
        .filter(|guardada| pendientes.iter().all(|p| p.id() != guardada.id()))
        .map(|guardada| (guardada.nombre().as_str(), guardada.id()))
        .collect();
    for pendiente in pendientes {
        if let Some(otra) = vistos.insert(pendiente.nombre().as_str(), pendiente.id())
            && otra != pendiente.id()
        {
            return Err(conflicto);
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct ContratistasMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<Contratista>,
}

impl RepositorioContratistas for ContratistasMemoria {
    async fn obtener(&self, id: ContratistaId) -> Result<Option<Contratista>, ErrorPersistencia> {
        self.almacen
            .leer(|datos| datos.contratistas.get(&id).cloned())
    }

    async fn obtener_por_cedula(
        &self,
        cedula: &Cedula,
    ) -> Result<Option<Contratista>, ErrorPersistencia> {
        self.almacen.leer(|datos| {
            datos
                .contratistas
                .values()
                .find(|contratista| contratista.cedula() == cedula)
                .cloned()
        })
    }

    async fn cedula_en_uso(
        &self,
        cedula: &Cedula,
        excepto: Option<ContratistaId>,
    ) -> Result<bool, ErrorPersistencia> {
        self.almacen.leer(|datos| {
            datos.contratistas.values().any(|contratista| {
                contratista.cedula() == cedula && Some(contratista.id()) != excepto
            })
        })
    }

    fn guardar(&mut self, contratista: &Contratista) {
        self.pendientes.push(contratista.clone());
    }
}

#[derive(Debug)]
pub struct EmpresasMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<Empresa>,
}

impl RepositorioEmpresas for EmpresasMemoria {
    async fn obtener(&self, id: EmpresaId) -> Result<Option<Empresa>, ErrorPersistencia> {
        self.almacen.leer(|datos| datos.empresas.get(&id).cloned())
    }

    async fn existe(&self, id: EmpresaId) -> Result<bool, ErrorPersistencia> {
        self.almacen.leer(|datos| datos.empresas.contains_key(&id))
    }

    async fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaId>,
    ) -> Result<bool, ErrorPersistencia> {
        self.almacen.leer(|datos| {
            datos
                .empresas
                .values()
                .any(|empresa| empresa.nombre() == nombre && Some(empresa.id()) != excepto)
        })
    }

    fn guardar(&mut self, empresa: &Empresa) {
        self.pendientes.push(empresa.clone());
    }
}

#[derive(Debug)]
pub struct PresenciasMemoria {
    almacen: AlmacenMemoria,
}

impl ConsultaPresencias for PresenciasMemoria {
    async fn esta_adentro(&self, contratista: ContratistaId) -> Result<bool, ErrorPersistencia> {
        self.almacen
            .leer(|datos| datos.adentro.contains(&contratista))
    }
}

#[derive(Debug)]
pub struct AuditoriaMemoria {
    pendientes: Vec<EntradaAuditoria>,
}

impl RegistroAuditoria for AuditoriaMemoria {
    fn anotar(&mut self, entrada: EntradaAuditoria) {
        self.pendientes.push(entrada);
    }
}

/// Reloj detenido en un instante fijo: las pruebas nunca dependen de la
/// hora real.
#[derive(Debug, Clone, Copy)]
pub struct RelojFijo {
    ahora: DateTime<Utc>,
    hoy: NaiveDate,
}

impl RelojFijo {
    pub const fn new(ahora: DateTime<Utc>, hoy: NaiveDate) -> Self {
        Self { ahora, hoy }
    }
}

impl Reloj for RelojFijo {
    fn ahora(&self) -> DateTime<Utc> {
        self.ahora
    }

    fn hoy(&self) -> NaiveDate {
        self.hoy
    }
}

/// IDs predecibles (1, 2, 3...) para que las pruebas sepan qué esperar.
#[derive(Debug, Clone, Default)]
pub struct IdsSecuenciales {
    siguiente: Arc<AtomicU64>,
}

impl IdsSecuenciales {
    pub fn new() -> Self {
        Self::default()
    }
}

impl GeneradorIds for IdsSecuenciales {
    fn nuevo(&self) -> Uuid {
        let numero = self.siguiente.fetch_add(1, Ordering::Relaxed) + 1;
        Uuid::from_u128(u128::from(numero))
    }
}
