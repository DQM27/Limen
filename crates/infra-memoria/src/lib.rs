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
//!   base (cédula de contratista, nombre de empresa), paso a paso como ella.
//!
//! La batería de `limen-pruebas-contrato` corre contra este doble y contra
//! `SurrealDB` para garantizar que se comportan igual.
//!
//! Además permite sembrar datos y simular fallas para las pruebas.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
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
    ids: IdsSecuenciales,
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

    /// Generador de IDs compartido por todo lo que usa este almacén: dos
    /// casos de uso de la misma prueba nunca reciben el mismo ID.
    pub fn ids(&self) -> IdsSecuenciales {
        self.ids.clone()
    }

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

impl UowMemoria {
    fn confirmar_ahora(self) -> Result<(), ErrorPersistencia> {
        let mut datos = self.almacen.bloquear();
        if let Some(error) = datos.falla_proxima_confirmacion.take() {
            return Err(error);
        }
        verificar_unicos(
            &datos.contratistas,
            &self.contratistas.pendientes,
            Contratista::id,
            |c| c.cedula().as_str(),
            Restriccion::CedulaContratista,
        )?;
        verificar_unicos(
            &datos.empresas,
            &self.empresas.pendientes,
            Empresa::id,
            |e| e.nombre().as_str(),
            Restriccion::NombreEmpresa,
        )?;

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

    fn confirmar(self) -> impl Future<Output = Result<(), ErrorPersistencia>> + Send {
        // Todo ocurre en memoria: el futuro ya nace resuelto.
        std::future::ready(self.confirmar_ahora())
    }
}

/// Lo que hace el índice único de la base: la unicidad se revisa en cada
/// escritura, en orden, contra el estado que va quedando (como `SurrealDB`,
/// sentencia por sentencia).
fn verificar_unicos<Id: Ord + Copy, E>(
    guardados: &BTreeMap<Id, E>,
    pendientes: &[E],
    id: impl Fn(&E) -> Id,
    clave: impl Fn(&E) -> &str,
    restriccion: Restriccion,
) -> Result<(), ErrorPersistencia> {
    let mut claves: BTreeMap<Id, &str> = guardados
        .iter()
        .map(|(id_guardado, entidad)| (*id_guardado, clave(entidad)))
        .collect();
    for pendiente in pendientes {
        let id_pendiente = id(pendiente);
        let clave_pendiente = clave(pendiente);
        let ocupada = claves
            .iter()
            .any(|(otro, valor)| *otro != id_pendiente && *valor == clave_pendiente);
        if ocupada {
            return Err(ErrorPersistencia::Conflicto(restriccion));
        }
        claves.insert(id_pendiente, clave_pendiente);
    }
    Ok(())
}

#[derive(Debug)]
pub struct ContratistasMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<Contratista>,
}

impl RepositorioContratistas for ContratistasMemoria {
    fn obtener(
        &self,
        id: ContratistaId,
    ) -> impl Future<Output = Result<Option<Contratista>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|datos| datos.contratistas.get(&id).cloned()),
        )
    }

    fn obtener_por_cedula(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<Option<Contratista>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|datos| {
            datos
                .contratistas
                .values()
                .find(|contratista| contratista.cedula() == cedula)
                .cloned()
        }))
    }

    fn cedula_en_uso(
        &self,
        cedula: &Cedula,
        excepto: Option<ContratistaId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|datos| {
            datos.contratistas.values().any(|contratista| {
                contratista.cedula() == cedula && Some(contratista.id()) != excepto
            })
        }))
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
    fn obtener(
        &self,
        id: EmpresaId,
    ) -> impl Future<Output = Result<Option<Empresa>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|datos| datos.empresas.get(&id).cloned()))
    }

    fn existe(
        &self,
        id: EmpresaId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|datos| datos.empresas.contains_key(&id)))
    }

    fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|datos| {
            datos
                .empresas
                .values()
                .any(|empresa| empresa.nombre() == nombre && Some(empresa.id()) != excepto)
        }))
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
    fn esta_adentro(
        &self,
        contratista: ContratistaId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|datos| datos.adentro.contains(&contratista)),
        )
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
