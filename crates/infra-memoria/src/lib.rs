//! Implementación en memoria de los puertos de Limen, para probar los casos
//! de uso sin base de datos.
//!
//! No es un juguete: se comporta como la base real en lo que importa a los
//! casos de uso, para que una prueba que pasa acá también pase contra
//! `SurrealDB`:
//!
//! - las lecturas ven sólo lo confirmado, nunca lo anotado en la misma
//!   Unit of Work;
//! - `confirmar()` aplica todo o nada;
//! - al confirmar se revisan las mismas restricciones que la base (cédula de
//!   contratista, nombre de empresa y de empresa proveedora, número de
//!   gafete, presencia única, gafete prestado y cédula de usuario), paso a
//!   paso como ella.
//!
//! La batería de `limen-pruebas-contrato` corre contra este doble y contra
//! `SurrealDB` para garantizar que se comportan igual.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc};
use limen_aplicacion::puertos::{
    CambioHistorial, Consultas, Contrasenas, EntradaAuditoria, EntradaHistorial, ErrorPersistencia,
    FabricaUnidadDeTrabajo, FilaContratista, GeneradorIds, IngresoAbierto, MovimientoHistorial,
    PersonaAdentro, RegistroAuditado, RegistroAuditoria, RegistroHechos, Reloj,
    RepositorioContratistas, RepositorioEmpresas, RepositorioEmpresasProveedoras,
    RepositorioGafetes, RepositorioIngresos, RepositorioIngresosCorreo,
    RepositorioIngresosProveedor, RepositorioIntentosInicio, RepositorioPersonalKof,
    RepositorioPresencias, RepositorioPrestamosKof, RepositorioReloj, RepositorioUsuarios,
    Restriccion, ResumenGafete, UnidadDeTrabajo,
};
use limen_dominio::busqueda::{Criterio, relevantes};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{Gafete, NumeroGafete, TipoGafete};
use limen_dominio::hecho::{Hecho, HechoId};
use limen_dominio::ingreso_contratista::{EntregaGafete, IngresoContratista, IngresoId};
use limen_dominio::ingreso_correo::{IngresoCorreo, IngresoCorreoId};
use limen_dominio::ingreso_proveedor::{IngresoProveedor, IngresoProveedorId};
use limen_dominio::operador::OperadorId;
use limen_dominio::personal_kof::{CodigoEmpleado, PersonalKof, PersonalKofId};
use limen_dominio::presencia::{Identidad, Via};
use limen_dominio::prestamo_kof::{PrestamoKof, PrestamoKofId};
use limen_dominio::reloj::LecturaReloj;
use limen_dominio::usuario::{ContrasenaNueva, HashContrasena, IntentosFallidos, Usuario};
use uuid::Uuid;

type ClaveGafete = (TipoGafete, NumeroGafete);

/// Lo guardado. Se clona entero al confirmar para aplicar todo o nada.
#[derive(Debug, Default, Clone)]
struct Contenido {
    contratistas: BTreeMap<ContratistaId, Contratista>,
    empresas: BTreeMap<EmpresaId, Empresa>,
    presencias: BTreeMap<String, Via>,
    gafetes: BTreeMap<ClaveGafete, Gafete>,
    prestamos: BTreeSet<ClaveGafete>,
    ingresos: BTreeMap<IngresoId, IngresoContratista>,
    empresas_proveedoras: BTreeMap<EmpresaProveedoraId, EmpresaProveedora>,
    ingresos_proveedor: BTreeMap<IngresoProveedorId, IngresoProveedor>,
    ingresos_correo: BTreeMap<IngresoCorreoId, IngresoCorreo>,
    personal_kof: BTreeMap<PersonalKofId, PersonalKof>,
    prestamos_kof: BTreeMap<PrestamoKofId, PrestamoKof>,
    /// Personas con un provisional sin devolver (clave natural).
    kof_con_prestamo: BTreeSet<PersonalKofId>,
    ultimo_movimiento: Option<DateTime<Utc>>,
    usuarios: BTreeMap<OperadorId, Usuario>,
    /// Intentos fallidos de inicio de sesión, por cédula.
    intentos_inicio: BTreeMap<String, IntentosFallidos>,
    auditoria: Vec<EntradaAuditoria>,
    hechos: BTreeMap<HechoId, Hecho>,
}

#[derive(Debug, Default)]
struct Datos {
    contenido: Contenido,
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

    fn leer<T>(&self, lectura: impl FnOnce(&Contenido) -> T) -> Result<T, ErrorPersistencia> {
        let datos = self.bloquear();
        datos
            .falla_lecturas
            .clone()
            .map_or_else(|| Ok(lectura(&datos.contenido)), Err)
    }

    fn sembrar(&self, cambio: impl FnOnce(&mut Contenido)) {
        cambio(&mut self.bloquear().contenido);
    }

    fn mirar<T>(&self, lectura: impl FnOnce(&Contenido) -> T) -> T {
        lectura(&self.bloquear().contenido)
    }

    // --- Para preparar las pruebas ---

    /// Generador de IDs compartido por todo lo que usa este almacén: dos
    /// casos de uso de la misma prueba nunca reciben el mismo ID.
    pub fn ids(&self) -> IdsSecuenciales {
        self.ids.clone()
    }

    pub fn sembrar_empresa(&self, empresa: Empresa) {
        self.sembrar(|c| {
            c.empresas.insert(empresa.id(), empresa);
        });
    }

    pub fn sembrar_empresa_proveedora(&self, empresa: EmpresaProveedora) {
        self.sembrar(|c| {
            c.empresas_proveedoras.insert(empresa.id(), empresa);
        });
    }

    pub fn sembrar_usuario(&self, usuario: Usuario) {
        self.sembrar(|c| {
            c.usuarios.insert(usuario.id(), usuario);
        });
    }

    pub fn sembrar_personal_kof(&self, persona: PersonalKof) {
        self.sembrar(|c| {
            c.personal_kof.insert(persona.id(), persona);
        });
    }

    pub fn sembrar_contratista(&self, contratista: Contratista) {
        self.sembrar(|c| {
            c.contratistas.insert(contratista.id(), contratista);
        });
    }

    pub fn sembrar_gafete(&self, gafete: Gafete) {
        self.sembrar(|c| {
            c.gafetes.insert((gafete.tipo(), gafete.numero()), gafete);
        });
    }

    /// Marca a una persona como adentro por una vía.
    pub fn marcar_adentro(&self, identidad: impl Into<Identidad>, via: Via) {
        let clave = identidad.into().clave();
        self.sembrar(|c| {
            c.presencias.insert(clave, via);
        });
    }

    pub fn fijar_ultimo_movimiento(&self, en: DateTime<Utc>) {
        self.sembrar(|c| c.ultimo_movimiento = Some(en));
    }

    /// La próxima confirmación falla con `error`, sin aplicar nada.
    pub fn fallar_proxima_confirmacion(&self, error: ErrorPersistencia) {
        self.bloquear().falla_proxima_confirmacion = Some(error);
    }

    /// Todas las lecturas fallan con `error` desde ahora.
    pub fn fallar_lecturas(&self, error: ErrorPersistencia) {
        self.bloquear().falla_lecturas = Some(error);
    }

    // --- Para revisar las pruebas ---

    pub fn contratistas(&self) -> Vec<Contratista> {
        self.mirar(|c| c.contratistas.values().cloned().collect())
    }

    pub fn empresas(&self) -> Vec<Empresa> {
        self.mirar(|c| c.empresas.values().cloned().collect())
    }

    pub fn gafetes(&self) -> Vec<Gafete> {
        self.mirar(|c| c.gafetes.values().cloned().collect())
    }

    pub fn ingresos(&self) -> Vec<IngresoContratista> {
        self.mirar(|c| c.ingresos.values().cloned().collect())
    }

    pub fn empresas_proveedoras(&self) -> Vec<EmpresaProveedora> {
        self.mirar(|c| c.empresas_proveedoras.values().cloned().collect())
    }

    pub fn ingresos_proveedor(&self) -> Vec<IngresoProveedor> {
        self.mirar(|c| c.ingresos_proveedor.values().cloned().collect())
    }

    pub fn ingresos_correo(&self) -> Vec<IngresoCorreo> {
        self.mirar(|c| c.ingresos_correo.values().cloned().collect())
    }

    pub fn personal_kof(&self) -> Vec<PersonalKof> {
        self.mirar(|c| c.personal_kof.values().cloned().collect())
    }

    pub fn usuarios(&self) -> Vec<Usuario> {
        self.mirar(|c| c.usuarios.values().cloned().collect())
    }

    /// Los intentos fallidos de inicio de sesión de una cédula.
    pub fn intentos_inicio(&self, cedula: &Cedula) -> Option<IntentosFallidos> {
        self.mirar(|c| c.intentos_inicio.get(cedula.as_str()).copied())
    }

    pub fn prestamos_kof(&self) -> Vec<PrestamoKof> {
        self.mirar(|c| c.prestamos_kof.values().cloned().collect())
    }

    /// Por qué vía está adentro una persona, si lo está.
    pub fn via_adentro(&self, identidad: impl Into<Identidad>) -> Option<Via> {
        let clave = identidad.into().clave();
        self.mirar(|c| c.presencias.get(&clave).copied())
    }

    pub fn prestado(&self, tipo: TipoGafete, numero: NumeroGafete) -> bool {
        self.mirar(|c| c.prestamos.contains(&(tipo, numero)))
    }

    pub fn ultimo_movimiento(&self) -> Option<DateTime<Utc>> {
        self.mirar(|c| c.ultimo_movimiento)
    }

    pub fn auditoria(&self) -> Vec<EntradaAuditoria> {
        self.mirar(|c| c.auditoria.clone())
    }

    /// Todos los hechos guardados, en el orden en que ocurrieron.
    pub fn hechos(&self) -> Vec<Hecho> {
        self.mirar(|c| c.hechos.values().cloned().collect())
    }

    /// Cuántas veces se confirmó con éxito una Unit of Work.
    pub fn confirmaciones(&self) -> usize {
        self.bloquear().confirmaciones
    }
}

impl Consultas for AlmacenMemoria {
    fn quienes_estan_adentro(
        &self,
    ) -> impl Future<Output = Result<Vec<PersonaAdentro>, ErrorPersistencia>> + Send {
        std::future::ready(self.leer(|c| {
            let mut adentro = Vec::new();
            for ingreso in c.ingresos.values().filter(|i| i.esta_abierto()) {
                let Some(contratista) = c.contratistas.get(&ingreso.contratista()) else {
                    continue;
                };
                adentro.push(PersonaAdentro {
                    ingreso: IngresoAbierto::Contratista(ingreso.id()),
                    identidad: Identidad::from(ingreso.cedula()),
                    nombre: contratista.nombre().clone(),
                    procedencia: c
                        .empresas
                        .get(&contratista.empresa())
                        .map(|empresa| empresa.nombre().to_string())
                        .unwrap_or_default(),
                    medio: Some(ingreso.medio().clone()),
                    gafete: ingreso.gafete(),
                    sin_gafete: ingreso.entrega_gafete() == EntregaGafete::SinGafete,
                    desde: ingreso.entrada().en,
                });
            }
            for ingreso in c.ingresos_proveedor.values().filter(|i| i.esta_abierto()) {
                adentro.push(PersonaAdentro {
                    ingreso: IngresoAbierto::Proveedor(ingreso.id()),
                    identidad: Identidad::from(ingreso.cedula()),
                    nombre: ingreso.visitante().nombre().clone(),
                    procedencia: c
                        .empresas_proveedoras
                        .get(&ingreso.empresa())
                        .map(|empresa| empresa.nombre().to_string())
                        .unwrap_or_default(),
                    medio: Some(ingreso.medio().clone()),
                    gafete: Some(ingreso.gafete()),
                    sin_gafete: false,
                    desde: ingreso.entrada().en,
                });
            }
            for ingreso in c.ingresos_correo.values().filter(|i| i.esta_abierto()) {
                adentro.push(PersonaAdentro {
                    ingreso: IngresoAbierto::Correo(ingreso.id()),
                    identidad: Identidad::from(ingreso.cedula()),
                    nombre: ingreso.visitante().nombre().clone(),
                    procedencia: ingreso.motivo().to_string(),
                    medio: Some(ingreso.medio().clone()),
                    gafete: Some(ingreso.gafete()),
                    sin_gafete: false,
                    desde: ingreso.entrada().en,
                });
            }
            for prestamo in c.prestamos_kof.values().filter(|p| p.esta_abierto()) {
                adentro.push(PersonaAdentro {
                    ingreso: IngresoAbierto::Kof(prestamo.id()),
                    identidad: prestamo.identidad(),
                    nombre: prestamo.nombre().clone(),
                    procedencia: "Personal KOF".to_owned(),
                    medio: None,
                    gafete: Some(prestamo.gafete()),
                    sin_gafete: false,
                    desde: prestamo.entrega().en,
                });
            }
            adentro.sort_by(|a, b| {
                b.desde
                    .cmp(&a.desde)
                    .then_with(|| a.identidad.to_string().cmp(&b.identidad.to_string()))
            });
            adentro
        }))
    }

    fn buscar_contratistas(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<Contratista>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.leer(|c| relevantes(criterio, c.contratistas.values().cloned(), limite)),
        )
    }

    fn buscar_personal_kof(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<PersonalKof>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.leer(|c| relevantes(criterio, c.personal_kof.values().cloned(), limite)),
        )
    }

    fn buscar_empresas(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<Empresa>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.leer(|c| relevantes(criterio, c.empresas.values().cloned(), limite)),
        )
    }

    fn buscar_empresas_proveedoras(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<EmpresaProveedora>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.leer(|c| relevantes(criterio, c.empresas_proveedoras.values().cloned(), limite)),
        )
    }

    fn historial_de(
        &self,
        registro: RegistroAuditado,
    ) -> impl Future<Output = Result<Vec<EntradaHistorial>, ErrorPersistencia>> + Send {
        std::future::ready(self.leer(|c| {
            let mut entradas: Vec<&EntradaAuditoria> = c
                .auditoria
                .iter()
                .filter(|entrada| entrada.registro == registro)
                .collect();
            // Igual que la base: el orden lo da el ID de cada entrada.
            entradas.sort_by_key(|entrada| entrada.id_entrada);
            entradas
                .into_iter()
                .map(|entrada| EntradaHistorial {
                    accion: entrada.accion,
                    cambios: entrada
                        .cambios
                        .iter()
                        .map(|cambio| CambioHistorial {
                            campo: cambio.campo.to_owned(),
                            antes: cambio.antes.clone(),
                            despues: cambio.despues.clone(),
                        })
                        .collect(),
                    operador: entrada.operador,
                    en: entrada.en,
                })
                .collect()
        }))
    }

    fn hechos_de(
        &self,
        registro: Uuid,
    ) -> impl Future<Output = Result<Vec<Hecho>, ErrorPersistencia>> + Send {
        // El mapa ya está ordenado por el ID del hecho, como en la base.
        std::future::ready(self.leer(|c| {
            c.hechos
                .values()
                .filter(|hecho| hecho.registro() == registro)
                .cloned()
                .collect()
        }))
    }

    fn listar_gafetes(
        &self,
        tipo: TipoGafete,
    ) -> impl Future<Output = Result<Vec<ResumenGafete>, ErrorPersistencia>> + Send {
        std::future::ready(self.leer(|c| {
            c.gafetes
                .iter()
                .filter(|((del_tipo, _), _)| *del_tipo == tipo)
                .map(|(clave, gafete)| ResumenGafete {
                    gafete: gafete.clone(),
                    prestado: c.prestamos.contains(clave),
                })
                .collect()
        }))
    }

    fn historial_de_ingresos(
        &self,
        desde: Option<DateTime<Utc>>,
        hasta: Option<DateTime<Utc>>,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<MovimientoHistorial>, ErrorPersistencia>> + Send {
        std::future::ready(self.leer(|c| {
            let en_el_rango = |entrada: DateTime<Utc>| {
                desde.is_none_or(|desde| entrada >= desde)
                    && hasta.is_none_or(|hasta| entrada < hasta)
            };
            let mut movimientos = Vec::new();
            for ingreso in c.ingresos.values().filter(|i| en_el_rango(i.entrada().en)) {
                let Some(contratista) = c.contratistas.get(&ingreso.contratista()) else {
                    continue;
                };
                movimientos.push(MovimientoHistorial {
                    ingreso: IngresoAbierto::Contratista(ingreso.id()),
                    identidad: Identidad::from(ingreso.cedula()),
                    nombre: contratista.nombre().clone(),
                    procedencia: c
                        .empresas
                        .get(&contratista.empresa())
                        .map(|empresa| empresa.nombre().to_string())
                        .unwrap_or_default(),
                    medio: Some(ingreso.medio().clone()),
                    gafete: ingreso.gafete(),
                    sin_gafete: ingreso.entrega_gafete() == EntregaGafete::SinGafete,
                    entrada: ingreso.entrada().en,
                    salida: ingreso.salida().map(|marca| marca.en),
                });
            }
            for ingreso in c
                .ingresos_proveedor
                .values()
                .filter(|i| en_el_rango(i.entrada().en))
            {
                movimientos.push(MovimientoHistorial {
                    ingreso: IngresoAbierto::Proveedor(ingreso.id()),
                    identidad: Identidad::from(ingreso.cedula()),
                    nombre: ingreso.visitante().nombre().clone(),
                    procedencia: c
                        .empresas_proveedoras
                        .get(&ingreso.empresa())
                        .map(|empresa| empresa.nombre().to_string())
                        .unwrap_or_default(),
                    medio: Some(ingreso.medio().clone()),
                    gafete: Some(ingreso.gafete()),
                    sin_gafete: false,
                    entrada: ingreso.entrada().en,
                    salida: ingreso.salida().map(|marca| marca.en),
                });
            }
            for ingreso in c
                .ingresos_correo
                .values()
                .filter(|i| en_el_rango(i.entrada().en))
            {
                movimientos.push(MovimientoHistorial {
                    ingreso: IngresoAbierto::Correo(ingreso.id()),
                    identidad: Identidad::from(ingreso.cedula()),
                    nombre: ingreso.visitante().nombre().clone(),
                    procedencia: ingreso.motivo().to_string(),
                    medio: Some(ingreso.medio().clone()),
                    gafete: Some(ingreso.gafete()),
                    sin_gafete: false,
                    entrada: ingreso.entrada().en,
                    salida: ingreso.salida().map(|marca| marca.en),
                });
            }
            for prestamo in c
                .prestamos_kof
                .values()
                .filter(|p| en_el_rango(p.entrega().en))
            {
                movimientos.push(MovimientoHistorial {
                    ingreso: IngresoAbierto::Kof(prestamo.id()),
                    identidad: prestamo.identidad(),
                    nombre: prestamo.nombre().clone(),
                    procedencia: "Personal KOF".to_owned(),
                    medio: None,
                    gafete: Some(prestamo.gafete()),
                    sin_gafete: false,
                    entrada: prestamo.entrega().en,
                    salida: prestamo.devolucion().map(|marca| marca.en),
                });
            }
            movimientos.sort_by(|a, b| {
                b.entrada
                    .cmp(&a.entrada)
                    .then_with(|| a.identidad.to_string().cmp(&b.identidad.to_string()))
            });
            movimientos.truncate(limite);
            movimientos
        }))
    }

    fn listar_contratistas(
        &self,
    ) -> impl Future<Output = Result<Vec<FilaContratista>, ErrorPersistencia>> + Send {
        std::future::ready(self.leer(|c| {
            let mut filas: Vec<FilaContratista> = c
                .contratistas
                .values()
                .filter_map(|contratista| {
                    // Nada se borra, así que la empresa siempre existe.
                    c.empresas
                        .get(&contratista.empresa())
                        .map(|empresa| FilaContratista {
                            contratista: contratista.clone(),
                            empresa: empresa.nombre().clone(),
                        })
                })
                .collect();
            filas.sort_by(|a, b| {
                a.contratista
                    .nombre()
                    .as_str()
                    .cmp(b.contratista.nombre().as_str())
                    .then_with(|| {
                        a.contratista
                            .cedula()
                            .as_str()
                            .cmp(b.contratista.cedula().as_str())
                    })
            });
            filas
        }))
    }

    fn contratistas_con_praind_hasta(
        &self,
        hasta: NaiveDate,
        limite: usize,
    ) -> impl Future<Output = Result<Vec<Contratista>, ErrorPersistencia>> + Send {
        std::future::ready(self.leer(|c| {
            let mut por_vencer: Vec<&Contratista> = c
                .contratistas
                .values()
                .filter(|persona| {
                    persona.tiene_acceso() && persona.fecha_vencimiento_praind() <= hasta
                })
                .collect();
            por_vencer.sort_by(|a, b| {
                a.fecha_vencimiento_praind()
                    .cmp(&b.fecha_vencimiento_praind())
                    .then_with(|| a.nombre().as_str().cmp(b.nombre().as_str()))
                    .then_with(|| a.cedula().as_str().cmp(b.cedula().as_str()))
            });
            por_vencer.into_iter().take(limite).cloned().collect()
        }))
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
                pendientes: Vec::new(),
            },
            gafetes: GafetesMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            ingresos: IngresosMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            empresas_proveedoras: EmpresasProveedorasMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            ingresos_proveedor: IngresosProveedorMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            ingresos_correo: IngresosCorreoMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            personal_kof: PersonalKofMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            prestamos_kof: PrestamosKofMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            reloj: RelojMemoria {
                almacen: self.clone(),
                pendiente: None,
            },
            usuarios: UsuariosMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            intentos_inicio: IntentosInicioMemoria {
                almacen: self.clone(),
                pendientes: Vec::new(),
            },
            auditoria: AuditoriaMemoria {
                pendientes: Vec::new(),
            },
            hechos: HechosMemoria {
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
    gafetes: GafetesMemoria,
    ingresos: IngresosMemoria,
    empresas_proveedoras: EmpresasProveedorasMemoria,
    ingresos_proveedor: IngresosProveedorMemoria,
    ingresos_correo: IngresosCorreoMemoria,
    personal_kof: PersonalKofMemoria,
    prestamos_kof: PrestamosKofMemoria,
    reloj: RelojMemoria,
    usuarios: UsuariosMemoria,
    intentos_inicio: IntentosInicioMemoria,
    auditoria: AuditoriaMemoria,
    hechos: HechosMemoria,
}

impl UowMemoria {
    fn confirmar_ahora(self) -> Result<(), ErrorPersistencia> {
        let mut datos = self.almacen.bloquear();
        if let Some(error) = datos.falla_proxima_confirmacion.take() {
            return Err(error);
        }
        // Se aplica todo sobre una copia; si algo choca, la copia se descarta.
        let mut nuevo = datos.contenido.clone();
        for empresa in self.empresas.pendientes {
            let ocupado = nuevo
                .empresas
                .values()
                .any(|otra| otra.id() != empresa.id() && otra.nombre() == empresa.nombre());
            if ocupado {
                return Err(ErrorPersistencia::Conflicto(Restriccion::NombreEmpresa));
            }
            nuevo.empresas.insert(empresa.id(), empresa);
        }
        for contratista in self.contratistas.pendientes {
            let ocupada = nuevo
                .contratistas
                .values()
                .any(|otro| otro.id() != contratista.id() && otro.cedula() == contratista.cedula());
            if ocupada {
                return Err(ErrorPersistencia::Conflicto(Restriccion::CedulaContratista));
            }
            nuevo.contratistas.insert(contratista.id(), contratista);
        }
        for empresa in self.empresas_proveedoras.pendientes {
            let ocupado = nuevo
                .empresas_proveedoras
                .values()
                .any(|otra| otra.id() != empresa.id() && otra.nombre() == empresa.nombre());
            if ocupado {
                return Err(ErrorPersistencia::Conflicto(
                    Restriccion::NombreEmpresaProveedora,
                ));
            }
            nuevo.empresas_proveedoras.insert(empresa.id(), empresa);
        }
        for operacion in self.gafetes.pendientes {
            aplicar_gafete(&mut nuevo, operacion)?;
        }
        for operacion in self.presencias.pendientes {
            aplicar_presencia(&mut nuevo, operacion)?;
        }
        for ingreso in self.ingresos.pendientes {
            nuevo.ingresos.insert(ingreso.id(), ingreso);
        }
        for ingreso in self.ingresos_proveedor.pendientes {
            nuevo.ingresos_proveedor.insert(ingreso.id(), ingreso);
        }
        for ingreso in self.ingresos_correo.pendientes {
            nuevo.ingresos_correo.insert(ingreso.id(), ingreso);
        }
        for persona in self.personal_kof.pendientes {
            let ocupado = nuevo
                .personal_kof
                .values()
                .any(|otra| otra.id() != persona.id() && otra.codigo() == persona.codigo());
            if ocupado {
                return Err(ErrorPersistencia::Conflicto(Restriccion::CodigoEmpleado));
            }
            nuevo.personal_kof.insert(persona.id(), persona);
        }
        for operacion in self.prestamos_kof.pendientes {
            aplicar_prestamo_kof(&mut nuevo, operacion)?;
        }
        if let Some(en) = self.reloj.pendiente {
            nuevo.ultimo_movimiento = Some(en);
        }
        aplicar_usuarios(&mut nuevo, self.usuarios.pendientes)?;
        for (cedula, intentos) in self.intentos_inicio.pendientes {
            match intentos {
                Some(intentos) => nuevo.intentos_inicio.insert(cedula, intentos),
                None => nuevo.intentos_inicio.remove(&cedula),
            };
        }
        nuevo.auditoria.extend(self.auditoria.pendientes);
        for hecho in self.hechos.pendientes {
            // Como `CREATE` en la base: un hecho nunca se reemplaza.
            if nuevo.hechos.contains_key(&hecho.id()) {
                return Err(ErrorPersistencia::Tecnica(format!(
                    "el hecho {} ya existe",
                    hecho.id()
                )));
            }
            nuevo.hechos.insert(hecho.id(), hecho);
        }

        datos.contenido = nuevo;
        datos.confirmaciones += 1;
        drop(datos);
        Ok(())
    }
}

fn aplicar_usuarios(
    contenido: &mut Contenido,
    usuarios: Vec<Usuario>,
) -> Result<(), ErrorPersistencia> {
    for usuario in usuarios {
        let ocupada = contenido
            .usuarios
            .values()
            .any(|otro| otro.id() != usuario.id() && otro.cedula() == usuario.cedula());
        if ocupada {
            return Err(ErrorPersistencia::Conflicto(Restriccion::CedulaUsuario));
        }
        contenido.usuarios.insert(usuario.id(), usuario);
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum OperacionGafete {
    Agregar(Gafete),
    Actualizar(Gafete),
    Prestar(ClaveGafete),
    Devolver(ClaveGafete),
}

fn aplicar_gafete(
    contenido: &mut Contenido,
    operacion: OperacionGafete,
) -> Result<(), ErrorPersistencia> {
    match operacion {
        OperacionGafete::Agregar(gafete) => {
            let clave = (gafete.tipo(), gafete.numero());
            if contenido.gafetes.contains_key(&clave) {
                return Err(ErrorPersistencia::Conflicto(Restriccion::NumeroGafete));
            }
            contenido.gafetes.insert(clave, gafete);
        }
        OperacionGafete::Actualizar(gafete) => {
            contenido
                .gafetes
                .insert((gafete.tipo(), gafete.numero()), gafete);
        }
        OperacionGafete::Prestar(clave) => {
            if !contenido.prestamos.insert(clave) {
                return Err(ErrorPersistencia::Conflicto(Restriccion::GafetePrestado));
            }
        }
        OperacionGafete::Devolver(clave) => {
            contenido.prestamos.remove(&clave);
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum OperacionPrestamoKof {
    Entrega(PrestamoKof),
    Devolucion(PrestamoKof),
}

fn aplicar_prestamo_kof(
    contenido: &mut Contenido,
    operacion: OperacionPrestamoKof,
) -> Result<(), ErrorPersistencia> {
    match operacion {
        OperacionPrestamoKof::Entrega(prestamo) => {
            if !contenido.kof_con_prestamo.insert(prestamo.personal()) {
                return Err(ErrorPersistencia::Conflicto(
                    Restriccion::PersonalKofConPrestamo,
                ));
            }
            contenido.prestamos_kof.insert(prestamo.id(), prestamo);
        }
        OperacionPrestamoKof::Devolucion(prestamo) => {
            contenido.kof_con_prestamo.remove(&prestamo.personal());
            contenido.prestamos_kof.insert(prestamo.id(), prestamo);
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum OperacionPresencia {
    Entrada(String, Via),
    Salida(String),
}

fn aplicar_presencia(
    contenido: &mut Contenido,
    operacion: OperacionPresencia,
) -> Result<(), ErrorPersistencia> {
    match operacion {
        OperacionPresencia::Entrada(cedula, via) => {
            if contenido.presencias.contains_key(&cedula) {
                return Err(ErrorPersistencia::Conflicto(Restriccion::PresenciaPersona));
            }
            contenido.presencias.insert(cedula, via);
        }
        OperacionPresencia::Salida(cedula) => {
            contenido.presencias.remove(&cedula);
        }
    }
    Ok(())
}

impl UnidadDeTrabajo for UowMemoria {
    type Contratistas = ContratistasMemoria;
    type Empresas = EmpresasMemoria;
    type Presencias = PresenciasMemoria;
    type Gafetes = GafetesMemoria;
    type Ingresos = IngresosMemoria;
    type EmpresasProveedoras = EmpresasProveedorasMemoria;
    type IngresosProveedor = IngresosProveedorMemoria;
    type IngresosCorreo = IngresosCorreoMemoria;
    type PersonalKof = PersonalKofMemoria;
    type PrestamosKof = PrestamosKofMemoria;
    type Reloj = RelojMemoria;
    type Usuarios = UsuariosMemoria;
    type IntentosInicio = IntentosInicioMemoria;
    type Auditoria = AuditoriaMemoria;
    type Hechos = HechosMemoria;

    fn contratistas(&mut self) -> &mut ContratistasMemoria {
        &mut self.contratistas
    }

    fn empresas(&mut self) -> &mut EmpresasMemoria {
        &mut self.empresas
    }

    fn presencias(&mut self) -> &mut PresenciasMemoria {
        &mut self.presencias
    }

    fn gafetes(&mut self) -> &mut GafetesMemoria {
        &mut self.gafetes
    }

    fn ingresos(&mut self) -> &mut IngresosMemoria {
        &mut self.ingresos
    }

    fn empresas_proveedoras(&mut self) -> &mut EmpresasProveedorasMemoria {
        &mut self.empresas_proveedoras
    }

    fn ingresos_proveedor(&mut self) -> &mut IngresosProveedorMemoria {
        &mut self.ingresos_proveedor
    }

    fn ingresos_correo(&mut self) -> &mut IngresosCorreoMemoria {
        &mut self.ingresos_correo
    }

    fn personal_kof(&mut self) -> &mut PersonalKofMemoria {
        &mut self.personal_kof
    }

    fn prestamos_kof(&mut self) -> &mut PrestamosKofMemoria {
        &mut self.prestamos_kof
    }

    fn reloj(&mut self) -> &mut RelojMemoria {
        &mut self.reloj
    }

    fn usuarios(&mut self) -> &mut UsuariosMemoria {
        &mut self.usuarios
    }

    fn intentos_inicio(&mut self) -> &mut IntentosInicioMemoria {
        &mut self.intentos_inicio
    }

    fn auditoria(&mut self) -> &mut AuditoriaMemoria {
        &mut self.auditoria
    }

    fn hechos(&mut self) -> &mut HechosMemoria {
        &mut self.hechos
    }

    fn confirmar(self) -> impl Future<Output = Result<(), ErrorPersistencia>> + Send {
        // Todo ocurre en memoria: el futuro ya nace resuelto.
        std::future::ready(self.confirmar_ahora())
    }
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
        std::future::ready(self.almacen.leer(|c| c.contratistas.get(&id).cloned()))
    }

    fn obtener_por_cedula(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<Option<Contratista>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.contratistas
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
        std::future::ready(self.almacen.leer(|c| {
            c.contratistas.values().any(|contratista| {
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
        std::future::ready(self.almacen.leer(|c| c.empresas.get(&id).cloned()))
    }

    fn existe(
        &self,
        id: EmpresaId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.empresas.contains_key(&id)))
    }

    fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.empresas
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
    pendientes: Vec<OperacionPresencia>,
}

impl RepositorioPresencias for PresenciasMemoria {
    fn via_adentro(
        &self,
        identidad: &Identidad,
    ) -> impl Future<Output = Result<Option<Via>, ErrorPersistencia>> + Send {
        let clave = identidad.clave();
        std::future::ready(self.almacen.leer(|c| c.presencias.get(&clave).copied()))
    }

    fn anotar_entrada(&mut self, identidad: &Identidad, via: Via, _desde: DateTime<Utc>) {
        self.pendientes
            .push(OperacionPresencia::Entrada(identidad.clave(), via));
    }

    fn anotar_salida(&mut self, identidad: &Identidad) {
        self.pendientes
            .push(OperacionPresencia::Salida(identidad.clave()));
    }
}

#[derive(Debug)]
pub struct GafetesMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<OperacionGafete>,
}

impl RepositorioGafetes for GafetesMemoria {
    fn obtener(
        &self,
        tipo: TipoGafete,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<Gafete>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|c| c.gafetes.get(&(tipo, numero)).cloned()),
        )
    }

    fn existentes(
        &self,
        tipo: TipoGafete,
        numeros: &[NumeroGafete],
    ) -> impl Future<Output = Result<Vec<NumeroGafete>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            let mut encontrados: Vec<NumeroGafete> = numeros
                .iter()
                .copied()
                .filter(|numero| c.gafetes.contains_key(&(tipo, *numero)))
                .collect();
            encontrados.sort_unstable();
            encontrados.dedup();
            encontrados
        }))
    }

    fn prestado(
        &self,
        tipo: TipoGafete,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.prestamos.contains(&(tipo, numero))))
    }

    fn agregar(&mut self, gafete: &Gafete) {
        self.pendientes
            .push(OperacionGafete::Agregar(gafete.clone()));
    }

    fn actualizar(&mut self, gafete: &Gafete) {
        self.pendientes
            .push(OperacionGafete::Actualizar(gafete.clone()));
    }

    fn anotar_prestamo(&mut self, tipo: TipoGafete, numero: NumeroGafete, _desde: DateTime<Utc>) {
        self.pendientes
            .push(OperacionGafete::Prestar((tipo, numero)));
    }

    fn anotar_devolucion(&mut self, tipo: TipoGafete, numero: NumeroGafete) {
        self.pendientes
            .push(OperacionGafete::Devolver((tipo, numero)));
    }
}

#[derive(Debug)]
pub struct IngresosMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<IngresoContratista>,
}

impl RepositorioIngresos for IngresosMemoria {
    fn obtener(
        &self,
        id: IngresoId,
    ) -> impl Future<Output = Result<Option<IngresoContratista>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.ingresos.get(&id).cloned()))
    }

    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<IngresoContratista>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.ingresos
                .values()
                .find(|ingreso| ingreso.esta_abierto() && ingreso.gafete() == Some(numero))
                .cloned()
        }))
    }

    fn guardar(&mut self, ingreso: &IngresoContratista) {
        self.pendientes.push(ingreso.clone());
    }
}

#[derive(Debug)]
pub struct EmpresasProveedorasMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<EmpresaProveedora>,
}

impl RepositorioEmpresasProveedoras for EmpresasProveedorasMemoria {
    fn obtener(
        &self,
        id: EmpresaProveedoraId,
    ) -> impl Future<Output = Result<Option<EmpresaProveedora>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|c| c.empresas_proveedoras.get(&id).cloned()),
        )
    }

    fn existe(
        &self,
        id: EmpresaProveedoraId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|c| c.empresas_proveedoras.contains_key(&id)),
        )
    }

    fn nombre_en_uso(
        &self,
        nombre: &NombreEmpresa,
        excepto: Option<EmpresaProveedoraId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.empresas_proveedoras
                .values()
                .any(|empresa| empresa.nombre() == nombre && Some(empresa.id()) != excepto)
        }))
    }

    fn guardar(&mut self, empresa: &EmpresaProveedora) {
        self.pendientes.push(empresa.clone());
    }
}

#[derive(Debug)]
pub struct IngresosProveedorMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<IngresoProveedor>,
}

impl RepositorioIngresosProveedor for IngresosProveedorMemoria {
    fn obtener(
        &self,
        id: IngresoProveedorId,
    ) -> impl Future<Output = Result<Option<IngresoProveedor>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|c| c.ingresos_proveedor.get(&id).cloned()),
        )
    }

    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<IngresoProveedor>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.ingresos_proveedor
                .values()
                .find(|ingreso| ingreso.esta_abierto() && ingreso.gafete() == numero)
                .cloned()
        }))
    }

    fn guardar(&mut self, ingreso: &IngresoProveedor) {
        self.pendientes.push(ingreso.clone());
    }
}

#[derive(Debug)]
pub struct IngresosCorreoMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<IngresoCorreo>,
}

impl RepositorioIngresosCorreo for IngresosCorreoMemoria {
    fn obtener(
        &self,
        id: IngresoCorreoId,
    ) -> impl Future<Output = Result<Option<IngresoCorreo>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.ingresos_correo.get(&id).cloned()))
    }

    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<IngresoCorreo>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.ingresos_correo
                .values()
                .find(|ingreso| ingreso.esta_abierto() && ingreso.gafete() == numero)
                .cloned()
        }))
    }

    fn guardar(&mut self, ingreso: &IngresoCorreo) {
        self.pendientes.push(ingreso.clone());
    }
}

#[derive(Debug)]
pub struct UsuariosMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<Usuario>,
}

impl RepositorioUsuarios for UsuariosMemoria {
    fn obtener(
        &self,
        id: OperadorId,
    ) -> impl Future<Output = Result<Option<Usuario>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.usuarios.get(&id).cloned()))
    }

    fn obtener_por_cedula(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<Option<Usuario>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.usuarios
                .values()
                .find(|usuario| usuario.cedula() == cedula)
                .cloned()
        }))
    }

    fn cedula_en_uso(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.usuarios
                .values()
                .any(|usuario| usuario.cedula() == cedula)
        }))
    }

    fn hay_usuarios(&self) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| !c.usuarios.is_empty()))
    }

    fn todos(&self) -> impl Future<Output = Result<Vec<Usuario>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            let mut usuarios: Vec<Usuario> = c.usuarios.values().cloned().collect();
            usuarios.sort_by(|a, b| {
                a.nombre()
                    .as_str()
                    .cmp(b.nombre().as_str())
                    .then_with(|| a.cedula().as_str().cmp(b.cedula().as_str()))
            });
            usuarios
        }))
    }

    fn guardar(&mut self, usuario: &Usuario) {
        self.pendientes.push(usuario.clone());
    }
}

#[derive(Debug)]
pub struct IntentosInicioMemoria {
    almacen: AlmacenMemoria,
    /// `None` borra los intentos de la cédula.
    pendientes: Vec<(String, Option<IntentosFallidos>)>,
}

impl RepositorioIntentosInicio for IntentosInicioMemoria {
    fn obtener(
        &self,
        cedula: &Cedula,
    ) -> impl Future<Output = Result<Option<IntentosFallidos>, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|c| c.intentos_inicio.get(cedula.as_str()).copied()),
        )
    }

    fn anotar(&mut self, cedula: &Cedula, intentos: IntentosFallidos) {
        self.pendientes
            .push((cedula.as_str().to_owned(), Some(intentos)));
    }

    fn borrar(&mut self, cedula: &Cedula) {
        self.pendientes.push((cedula.as_str().to_owned(), None));
    }
}

/// Cifrador de mentira para las pruebas: instantáneo y predecible. El de
/// verdad (Argon2id) está en `infra-plataforma`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ContrasenasFalsas;

impl ContrasenasFalsas {
    /// El "hash" que este cifrador le da a `texto`.
    pub fn hash_de(texto: &str) -> HashContrasena {
        HashContrasena::desde_texto(format!("falso:{texto}"))
    }
}

impl Contrasenas for ContrasenasFalsas {
    fn cifrar(&self, contrasena: &ContrasenaNueva) -> Result<HashContrasena, String> {
        Ok(Self::hash_de(contrasena.as_str()))
    }

    fn verificar(&self, contrasena: &str, hash: Option<&HashContrasena>) -> bool {
        hash.is_some_and(|hash| hash == &Self::hash_de(contrasena))
    }
}

#[derive(Debug)]
pub struct PersonalKofMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<PersonalKof>,
}

impl RepositorioPersonalKof for PersonalKofMemoria {
    fn obtener(
        &self,
        id: PersonalKofId,
    ) -> impl Future<Output = Result<Option<PersonalKof>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.personal_kof.get(&id).cloned()))
    }

    fn codigo_en_uso(
        &self,
        codigo: &CodigoEmpleado,
        excepto: Option<PersonalKofId>,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.personal_kof
                .values()
                .any(|persona| persona.codigo() == codigo && Some(persona.id()) != excepto)
        }))
    }

    fn guardar(&mut self, persona: &PersonalKof) {
        self.pendientes.push(persona.clone());
    }
}

#[derive(Debug)]
pub struct PrestamosKofMemoria {
    almacen: AlmacenMemoria,
    pendientes: Vec<OperacionPrestamoKof>,
}

impl RepositorioPrestamosKof for PrestamosKofMemoria {
    fn obtener(
        &self,
        id: PrestamoKofId,
    ) -> impl Future<Output = Result<Option<PrestamoKof>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.prestamos_kof.get(&id).cloned()))
    }

    fn tiene_abierto(
        &self,
        personal: PersonalKofId,
    ) -> impl Future<Output = Result<bool, ErrorPersistencia>> + Send {
        std::future::ready(
            self.almacen
                .leer(|c| c.kof_con_prestamo.contains(&personal)),
        )
    }

    fn abierto_con_gafete(
        &self,
        numero: NumeroGafete,
    ) -> impl Future<Output = Result<Option<PrestamoKof>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| {
            c.prestamos_kof
                .values()
                .find(|prestamo| prestamo.esta_abierto() && prestamo.gafete() == numero)
                .cloned()
        }))
    }

    fn anotar_entrega(&mut self, prestamo: &PrestamoKof) {
        self.pendientes
            .push(OperacionPrestamoKof::Entrega(prestamo.clone()));
    }

    fn anotar_devolucion(&mut self, prestamo: &PrestamoKof) {
        self.pendientes
            .push(OperacionPrestamoKof::Devolucion(prestamo.clone()));
    }
}

#[derive(Debug)]
pub struct RelojMemoria {
    almacen: AlmacenMemoria,
    pendiente: Option<DateTime<Utc>>,
}

impl RepositorioReloj for RelojMemoria {
    fn ultimo_movimiento(
        &self,
    ) -> impl Future<Output = Result<Option<DateTime<Utc>>, ErrorPersistencia>> + Send {
        std::future::ready(self.almacen.leer(|c| c.ultimo_movimiento))
    }

    fn anotar_movimiento(&mut self, en: DateTime<Utc>) {
        self.pendiente = Some(en);
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

#[derive(Debug)]
pub struct HechosMemoria {
    pendientes: Vec<Hecho>,
}

impl RegistroHechos for HechosMemoria {
    fn anotar(&mut self, hecho: Hecho) {
        self.pendientes.push(hecho);
    }
}

/// Reloj detenido en un instante fijo: las pruebas nunca dependen de la
/// hora real. Por omisión su hora está comprobada (margen 0), como la de un
/// equipo recién sincronizado.
#[derive(Debug, Clone, Copy)]
pub struct RelojFijo {
    ahora: DateTime<Utc>,
    hoy: NaiveDate,
    comprobado: bool,
}

impl RelojFijo {
    pub const fn new(ahora: DateTime<Utc>, hoy: NaiveDate) -> Self {
        Self {
            ahora,
            hoy,
            comprobado: true,
        }
    }

    /// El mismo reloj, pero sin haber podido comprobar su hora contra una
    /// fuente externa (sin red desde que arrancó).
    #[must_use]
    pub const fn sin_comprobar(self) -> Self {
        Self {
            comprobado: false,
            ..self
        }
    }
}

impl Reloj for RelojFijo {
    fn ahora(&self) -> DateTime<Utc> {
        self.ahora
    }

    fn hoy(&self) -> NaiveDate {
        self.hoy
    }

    /// Costa Rica va seis horas detrás de UTC todo el año.
    fn inicio_del_dia(&self, fecha: NaiveDate) -> DateTime<Utc> {
        let medianoche = Utc.from_utc_datetime(&fecha.and_time(NaiveTime::MIN));
        medianoche
            + TimeDelta::try_hours(HORAS_DE_COSTA_RICA_DETRAS_DE_UTC)
                .unwrap_or_else(TimeDelta::zero)
    }

    fn lectura(&self) -> LecturaReloj {
        LecturaReloj {
            instante: self.ahora,
            margen_ms: self.comprobado.then_some(0),
            hora_equipo: self.ahora,
        }
    }
}

const HORAS_DE_COSTA_RICA_DETRAS_DE_UTC: i64 = 6;

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
