//! Orquesta la carga: lee cada tabla del volcado, la traduce con las reglas
//! del dominio (`traduccion`) y la guarda por los puertos de `aplicacion`, en
//! lotes y con la auditoría del alta (B11). Funciona con cualquier adaptador:
//! las pruebas lo corren con los dobles en memoria y datos inventados.
//!
//! Además de las reglas de cada fila, cuida las que cruzan filas, que el
//! dominio decide con "hechos" que normalmente trae el caso de uso: cédula,
//! nombre o código repetidos, empresa que no existe, una persona adentro por
//! dos vías a la vez y un gafete prestado dos veces. Lo que las rompe se
//! rechaza con su código en lugar de hacer fallar la carga entera.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{
    AccionAuditada, EntradaAuditoria, ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds,
    RegistroAuditado, RegistroAuditoria, RepositorioContratistas, RepositorioEmpresas,
    RepositorioEmpresasProveedoras, RepositorioGafetes, RepositorioIngresos,
    RepositorioIngresosCorreo, RepositorioIngresosProveedor, RepositorioPersonalKof,
    RepositorioPresencias, RepositorioPrestamosKof, RepositorioReloj, UnidadDeTrabajo,
};
use limen_aplicacion::sesion::{OperadorId, Sesion};
use limen_dominio::auditoria::CambioCampo;
use limen_dominio::contratista::Contratista;
use limen_dominio::gafete::TipoGafete;
use limen_dominio::personal_kof::PersonalKof;
use limen_dominio::presencia::{Identidad, Via};
use limen_dominio::prestamo_kof::{PrestamoKof, PrestamoKofGuardado};
use uuid::Uuid;

use crate::lectura::{ErrorLectura, Fila, Tabla};
use crate::traduccion::{self, CatalogoProveedoras, OPERADOR_IMPORTADOR, Traducido};

/// Cuántas filas se guardan por transacción.
const TAMANO_LOTE: usize = 150;

/// Cuántos identificadores de filas rechazadas se muestran por motivo.
const MUESTRA_DE_RECHAZOS: usize = 5;

// --- Datos de entrada ---

/// El SQL de cada tabla del volcado; una tabla ausente se trata como vacía.
#[derive(Debug, Default, Clone)]
pub struct Datos {
    pub empresas: Option<String>,
    pub contratistas: Option<String>,
    pub gafetes: Option<String>,
    pub empresas_proveedor: Option<String>,
    pub personal_kof: Option<String>,
    pub ingresos_contratista: Option<String>,
    pub ingresos_proveedor: Option<String>,
    pub ingresos_correo: Option<String>,
    pub prestamos_kof: Option<String>,
}

impl Datos {
    /// Lee los archivos `NN_tabla.sql` de la carpeta del volcado.
    pub fn desde_carpeta(carpeta: &Path) -> Result<Self, ErrorImportacion> {
        let leer = |nombre: &str| -> Result<Option<String>, ErrorImportacion> {
            match fs::read_to_string(carpeta.join(nombre)) {
                Ok(texto) => Ok(Some(texto)),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
                Err(error) => Err(ErrorImportacion::Archivo(format!("{nombre}: {error}"))),
            }
        };
        Ok(Self {
            empresas: leer("01_empresas.sql")?,
            contratistas: leer("02_contratistas.sql")?,
            gafetes: leer("03_gafetes.sql")?,
            empresas_proveedor: leer("04_empresas_proveedor.sql")?,
            personal_kof: leer("05_personal_kof.sql")?,
            ingresos_contratista: leer("06_ingresos_contratista.sql")?,
            ingresos_proveedor: leer("07_ingresos_proveedor.sql")?,
            ingresos_correo: leer("08_ingresos_correo.sql")?,
            prestamos_kof: leer("09_prestamos_gafete_kof.sql")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorImportacion {
    #[error("no se pudo leer el archivo {0}")]
    Archivo(String),
    #[error("la tabla {tabla} no tiene el formato esperado: {error}")]
    Lectura {
        tabla: &'static str,
        error: ErrorLectura,
    },
    #[error(
        "no se pudo guardar la tabla {tabla}: {error}. Si la base ya tenía datos, bórrela y \
         vuelva a importar"
    )]
    Persistencia {
        tabla: &'static str,
        error: ErrorPersistencia,
    },
}

// --- Resultado ---

/// Una fila que no se cargó: el identificador de Lattis y un código. Nunca
/// datos de la persona.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rechazo {
    pub id: String,
    pub motivo: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumenTabla {
    pub tabla: &'static str,
    pub leidas: usize,
    pub cargadas: usize,
    pub rechazos: Vec<Rechazo>,
    /// Correcciones que hubo que hacer para aceptar filas, y cuántas veces.
    pub ajustes: BTreeMap<&'static str, usize>,
}

impl ResumenTabla {
    const fn nueva(tabla: &'static str) -> Self {
        Self {
            tabla,
            leidas: 0,
            cargadas: 0,
            rechazos: Vec::new(),
            ajustes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resumen {
    pub tablas: Vec<ResumenTabla>,
}

impl Resumen {
    pub fn tabla(&self, nombre: &str) -> Option<&ResumenTabla> {
        self.tablas.iter().find(|tabla| tabla.tabla == nombre)
    }
}

impl fmt::Display for Resumen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for tabla in &self.tablas {
            write!(
                f,
                "{}: leídas {}, cargadas {}",
                tabla.tabla, tabla.leidas, tabla.cargadas
            )?;
            if !tabla.rechazos.is_empty() {
                write!(f, ", rechazadas {}", tabla.rechazos.len())?;
            }
            writeln!(f)?;
            for (ajuste, veces) in &tabla.ajustes {
                writeln!(f, "    ajuste {ajuste}: {veces}")?;
            }
            let mut por_motivo: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for rechazo in &tabla.rechazos {
                por_motivo
                    .entry(&rechazo.motivo)
                    .or_default()
                    .push(&rechazo.id);
            }
            for (motivo, ids) in por_motivo {
                let muestra: Vec<&str> = ids.iter().copied().take(MUESTRA_DE_RECHAZOS).collect();
                writeln!(
                    f,
                    "    rechazo {motivo} ×{} (ids de Lattis: {}{})",
                    ids.len(),
                    muestra.join(", "),
                    if ids.len() > MUESTRA_DE_RECHAZOS {
                        ", …"
                    } else {
                        ""
                    }
                )?;
            }
        }
        Ok(())
    }
}

// --- Traducción de una tabla ---

struct Traducidas<T> {
    aceptadas: Vec<T>,
    resumen: ResumenTabla,
}

/// Lee la tabla y traduce cada fila; lo que se rechaza queda en el resumen.
fn traducir<T>(
    tabla: &'static str,
    sql: Option<&str>,
    mut traductor: impl FnMut(Fila<'_>) -> Traducido<T>,
) -> Result<Traducidas<T>, ErrorImportacion> {
    let mut resumen = ResumenTabla::nueva(tabla);
    let mut aceptadas = Vec::new();
    let Some(sql) = sql.filter(|sql| !sql.trim().is_empty()) else {
        return Ok(Traducidas { aceptadas, resumen });
    };
    let leida = Tabla::leer(sql).map_err(|error| ErrorImportacion::Lectura { tabla, error })?;
    resumen.leidas = leida.len();
    for fila in leida.filas() {
        match traductor(fila) {
            Ok((valor, ajuste)) => {
                if let Some(ajuste) = ajuste {
                    *resumen.ajustes.entry(ajuste).or_default() += 1;
                }
                aceptadas.push(valor);
            }
            Err(motivo) => resumen.rechazos.push(Rechazo {
                id: fila.opcional("id").ok().flatten().unwrap_or("?").to_owned(),
                motivo,
            }),
        }
    }
    resumen.cargadas = aceptadas.len();
    Ok(Traducidas { aceptadas, resumen })
}

/// El instante más reciente de una entrada y su salida (si ya salió).
fn instantes_de(
    entrada: DateTime<Utc>,
    salida: Option<DateTime<Utc>>,
) -> impl Iterator<Item = DateTime<Utc>> {
    std::iter::once(entrada).chain(salida)
}

/// Lo que ya está adentro o prestado, para no cargar dos veces lo que la base
/// exige que sea único (una persona adentro, un gafete prestado).
#[derive(Debug, Default)]
struct Abiertos {
    personas: HashSet<String>,
    gafetes: HashSet<(TipoGafete, u32)>,
}

impl Abiertos {
    /// Anota a la persona y su gafete; `Err` si alguno ya estaba ocupado.
    fn ocupar(
        &mut self,
        identidad: &Identidad,
        gafete: Option<(TipoGafete, u32)>,
    ) -> Result<(), String> {
        if self.personas.contains(&identidad.clave()) {
            return Err("ya_estaba_adentro_por_otra_via".to_owned());
        }
        if gafete.is_some_and(|gafete| self.gafetes.contains(&gafete)) {
            return Err("gafete_ya_prestado".to_owned());
        }
        self.personas.insert(identidad.clave());
        self.gafetes.extend(gafete);
        Ok(())
    }
}

// --- Carga ---

struct Cargador<'a, F, G> {
    fabrica: &'a F,
    ids: &'a G,
    ahora: DateTime<Utc>,
    sesion: Sesion,
}

impl<F: FabricaUnidadDeTrabajo, G: GeneradorIds> Cargador<'_, F, G> {
    /// Guarda las entidades por lotes: cada lote es una transacción.
    async fn guardar<T>(
        &self,
        tabla: &'static str,
        entidades: &[T],
        mut escribir: impl FnMut(&mut F::Uow, &T) + Send,
    ) -> Result<(), ErrorImportacion>
    where
        T: Sync,
    {
        for lote in entidades.chunks(TAMANO_LOTE) {
            let mut uow = self.fabrica.nueva();
            for entidad in lote {
                escribir(&mut uow, entidad);
            }
            uow.confirmar()
                .await
                .map_err(|error| ErrorImportacion::Persistencia { tabla, error })?;
        }
        Ok(())
    }

    /// El alta de cada registro queda auditada, como cualquier otro cambio.
    fn auditar(&self, uow: &mut F::Uow, registro: RegistroAuditado, cambios: Vec<CambioCampo>) {
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            registro,
            AccionAuditada::Alta,
            cambios,
            &self.sesion,
            self.ahora,
        ));
    }

    async fn empresas(
        &self,
        sql: Option<&str>,
    ) -> Result<(HashSet<Uuid>, ResumenTabla), ErrorImportacion> {
        let mut nombres = HashSet::new();
        let leidas = traducir("empresas", sql, |fila| {
            let empresa = traduccion::empresa(fila)?;
            if !nombres.insert(empresa.nombre().as_str().to_owned()) {
                return Err("nombre_repetido".to_owned());
            }
            Ok((empresa, None))
        })?;
        self.guardar("empresas", &leidas.aceptadas, |uow, empresa| {
            uow.empresas().guardar(empresa);
            self.auditar(
                uow,
                RegistroAuditado::Empresa(empresa.id()),
                empresa.cambios_de_alta(),
            );
        })
        .await?;
        let ids = leidas.aceptadas.iter().map(|e| e.id().uuid()).collect();
        Ok((ids, leidas.resumen))
    }

    async fn gafetes(&self, sql: Option<&str>) -> Result<ResumenTabla, ErrorImportacion> {
        let mut vistos = HashSet::new();
        let leidas = traducir("gafetes", sql, |fila| {
            let gafete = traduccion::gafete(fila)?;
            if !vistos.insert((gafete.tipo(), gafete.numero().valor())) {
                return Err("gafete_repetido".to_owned());
            }
            Ok((gafete, None))
        })?;
        self.guardar("gafetes", &leidas.aceptadas, |uow, gafete| {
            uow.gafetes().agregar(gafete);
            self.auditar(
                uow,
                RegistroAuditado::Gafete(gafete.tipo(), gafete.numero()),
                gafete.cambios_de_alta(),
            );
        })
        .await?;
        Ok(leidas.resumen)
    }

    async fn empresas_proveedoras(
        &self,
        sql: Option<&str>,
    ) -> Result<(CatalogoProveedoras, ResumenTabla), ErrorImportacion> {
        let mut nombres = HashSet::new();
        let leidas = traducir("empresas_proveedor", sql, |fila| {
            let empresa = traduccion::empresa_proveedora(fila)?;
            if !nombres.insert(empresa.nombre().as_str().to_owned()) {
                return Err("nombre_repetido".to_owned());
            }
            Ok((empresa, None))
        })?;
        self.guardar("empresas_proveedor", &leidas.aceptadas, |uow, empresa| {
            uow.empresas_proveedoras().guardar(empresa);
            self.auditar(
                uow,
                RegistroAuditado::EmpresaProveedora(empresa.id()),
                empresa.cambios_de_alta(),
            );
        })
        .await?;
        let mut catalogo = CatalogoProveedoras::default();
        for empresa in &leidas.aceptadas {
            catalogo.agregar(empresa);
        }
        Ok((catalogo, leidas.resumen))
    }

    async fn personal_kof(
        &self,
        sql: Option<&str>,
    ) -> Result<(HashMap<Uuid, PersonalKof>, ResumenTabla), ErrorImportacion> {
        let mut codigos = HashSet::new();
        let leidas = traducir("personal_kof", sql, |fila| {
            let persona = traduccion::personal_kof(fila)?;
            if !codigos.insert(persona.codigo().as_str().to_owned()) {
                return Err("codigo_repetido".to_owned());
            }
            Ok((persona, None))
        })?;
        self.guardar("personal_kof", &leidas.aceptadas, |uow, persona| {
            uow.personal_kof().guardar(persona);
            self.auditar(
                uow,
                RegistroAuditado::PersonalKof(persona.id()),
                persona.cambios_de_alta(),
            );
        })
        .await?;
        let por_id = leidas
            .aceptadas
            .into_iter()
            .map(|persona| (persona.id().uuid(), persona))
            .collect();
        Ok((por_id, leidas.resumen))
    }

    async fn contratistas(
        &self,
        sql: Option<&str>,
        empresas: &HashSet<Uuid>,
    ) -> Result<(HashMap<Uuid, Contratista>, ResumenTabla), ErrorImportacion> {
        let mut cedulas = HashSet::new();
        let leidas = traducir("contratistas", sql, |fila| {
            let (contratista, ajuste) = traduccion::contratista(fila)?;
            if !empresas.contains(&contratista.empresa().uuid()) {
                return Err("empresa_no_existe".to_owned());
            }
            if !cedulas.insert(contratista.cedula().as_str().to_owned()) {
                return Err("cedula_repetida".to_owned());
            }
            Ok((contratista, ajuste))
        })?;
        self.guardar("contratistas", &leidas.aceptadas, |uow, contratista| {
            uow.contratistas().guardar(contratista);
            self.auditar(
                uow,
                RegistroAuditado::Contratista(contratista.id()),
                contratista.cambios_de_alta(),
            );
        })
        .await?;
        let por_id = leidas
            .aceptadas
            .into_iter()
            .map(|contratista| (contratista.id().uuid(), contratista))
            .collect();
        Ok((por_id, leidas.resumen))
    }

    async fn ingresos_contratista(
        &self,
        sql: Option<&str>,
        contratistas: &HashMap<Uuid, Contratista>,
        abiertos: &mut Abiertos,
    ) -> Result<(Option<DateTime<Utc>>, ResumenTabla), ErrorImportacion> {
        let leidas = traducir("ingresos_contratista", sql, |fila| {
            let contratista = contratistas
                .get(&traduccion::uuid_de(fila, "contratista_id")?)
                .ok_or_else(|| "contratista_no_importado".to_owned())?;
            let (ingreso, ajuste) = traduccion::ingreso_contratista(fila, contratista)?;
            if ingreso.esta_abierto() {
                abiertos.ocupar(
                    &Identidad::from(ingreso.cedula()),
                    ingreso
                        .gafete()
                        .map(|numero| (TipoGafete::Contratista, numero.valor())),
                )?;
            }
            Ok((ingreso, ajuste))
        })?;
        self.guardar("ingresos_contratista", &leidas.aceptadas, |uow, ingreso| {
            uow.ingresos().guardar(ingreso);
            if ingreso.esta_abierto() {
                let desde = ingreso.entrada().en;
                uow.presencias().anotar_entrada(
                    &Identidad::from(ingreso.cedula()),
                    Via::Contratista,
                    desde,
                );
                if let Some(numero) = ingreso.gafete() {
                    uow.gafetes()
                        .anotar_prestamo(TipoGafete::Contratista, numero, desde);
                }
            }
        })
        .await?;
        let ultimo = leidas
            .aceptadas
            .iter()
            .flat_map(|ingreso| instantes_de(ingreso.entrada().en, ingreso.salida().map(|s| s.en)))
            .max();
        Ok((ultimo, leidas.resumen))
    }

    async fn ingresos_proveedor(
        &self,
        sql: Option<&str>,
        empresas: &CatalogoProveedoras,
        abiertos: &mut Abiertos,
    ) -> Result<(Option<DateTime<Utc>>, ResumenTabla), ErrorImportacion> {
        let leidas = traducir("ingresos_proveedor", sql, |fila| {
            let (ingreso, ajuste) = traduccion::ingreso_proveedor(fila, empresas)?;
            if ingreso.esta_abierto() {
                abiertos.ocupar(
                    &Identidad::from(ingreso.cedula()),
                    Some((TipoGafete::Proveedor, ingreso.gafete().valor())),
                )?;
            }
            Ok((ingreso, ajuste))
        })?;
        self.guardar("ingresos_proveedor", &leidas.aceptadas, |uow, ingreso| {
            uow.ingresos_proveedor().guardar(ingreso);
            if ingreso.esta_abierto() {
                let desde = ingreso.entrada().en;
                uow.presencias().anotar_entrada(
                    &Identidad::from(ingreso.cedula()),
                    Via::Proveedor,
                    desde,
                );
                uow.gafetes()
                    .anotar_prestamo(TipoGafete::Proveedor, ingreso.gafete(), desde);
            }
        })
        .await?;
        let ultimo = leidas
            .aceptadas
            .iter()
            .flat_map(|ingreso| instantes_de(ingreso.entrada().en, ingreso.salida().map(|s| s.en)))
            .max();
        Ok((ultimo, leidas.resumen))
    }

    async fn ingresos_correo(
        &self,
        sql: Option<&str>,
        abiertos: &mut Abiertos,
    ) -> Result<(Option<DateTime<Utc>>, ResumenTabla), ErrorImportacion> {
        let leidas = traducir("ingresos_correo", sql, |fila| {
            let (ingreso, ajuste) = traduccion::ingreso_correo(fila)?;
            if ingreso.esta_abierto() {
                abiertos.ocupar(
                    &Identidad::from(ingreso.cedula()),
                    Some((TipoGafete::Visita, ingreso.gafete().valor())),
                )?;
            }
            Ok((ingreso, ajuste))
        })?;
        self.guardar("ingresos_correo", &leidas.aceptadas, |uow, ingreso| {
            uow.ingresos_correo().guardar(ingreso);
            if ingreso.esta_abierto() {
                let desde = ingreso.entrada().en;
                uow.presencias().anotar_entrada(
                    &Identidad::from(ingreso.cedula()),
                    Via::Correo,
                    desde,
                );
                uow.gafetes()
                    .anotar_prestamo(TipoGafete::Visita, ingreso.gafete(), desde);
            }
        })
        .await?;
        let ultimo = leidas
            .aceptadas
            .iter()
            .flat_map(|ingreso| instantes_de(ingreso.entrada().en, ingreso.salida().map(|s| s.en)))
            .max();
        Ok((ultimo, leidas.resumen))
    }

    async fn prestamos_kof(
        &self,
        sql: Option<&str>,
        personal: &HashMap<Uuid, PersonalKof>,
        abiertos: &mut Abiertos,
    ) -> Result<(Option<DateTime<Utc>>, ResumenTabla), ErrorImportacion> {
        let leidas = traducir("prestamos_kof", sql, |fila| {
            let prestamo = traduccion::prestamo_kof(fila, personal)?;
            if prestamo.devolucion.is_none() {
                abiertos.ocupar(
                    &Identidad::from(&prestamo.codigo),
                    Some((TipoGafete::ProvisionalKof, prestamo.gafete.valor())),
                )?;
            }
            Ok((prestamo, None))
        })?;
        // Los cerrados primero y en orden cronológico: la devolución de un
        // préstamo quita la marca "tiene un préstamo abierto" de esa persona,
        // así que no puede llegar después de la entrega de uno abierto.
        let Traducidas {
            mut aceptadas,
            resumen,
        } = leidas;
        aceptadas.sort_by_key(|prestamo| (prestamo.devolucion.is_none(), prestamo.entrega.en));
        self.guardar("prestamos_kof", &aceptadas, |uow, guardado| {
            escribir_prestamo(uow, guardado);
        })
        .await?;
        let ultimo = aceptadas
            .iter()
            .flat_map(|prestamo| {
                instantes_de(prestamo.entrega.en, prestamo.devolucion.map(|d| d.en))
            })
            .max();
        Ok((ultimo, resumen))
    }

    /// El reloj no puede quedar antes del último movimiento cargado (E5).
    async fn fijar_reloj(&self, ultimo: Option<DateTime<Utc>>) -> Result<(), ErrorImportacion> {
        let Some(ultimo) = ultimo else {
            return Ok(());
        };
        self.guardar("reloj", &[ultimo], |uow, en| {
            uow.reloj().anotar_movimiento(*en);
        })
        .await
    }
}

/// Un préstamo KOF se carga como lo haría la app: la entrega, y si ya se
/// devolvió, la devolución (que quita la marca de "tiene préstamo abierto").
/// Un préstamo abierto deja a la persona adentro y el gafete prestado.
fn escribir_prestamo<U: UnidadDeTrabajo>(uow: &mut U, guardado: &PrestamoKofGuardado) {
    let abierto = PrestamoKof::restaurar(PrestamoKofGuardado {
        devolucion: None,
        ..guardado.clone()
    });
    uow.prestamos_kof().anotar_entrega(&abierto);
    if guardado.devolucion.is_some() {
        uow.prestamos_kof()
            .anotar_devolucion(&PrestamoKof::restaurar(guardado.clone()));
    } else {
        uow.presencias()
            .anotar_entrada(&abierto.identidad(), Via::Kof, guardado.entrega.en);
        uow.gafetes().anotar_prestamo(
            TipoGafete::ProvisionalKof,
            guardado.gafete,
            guardado.entrega.en,
        );
    }
}

/// Carga el volcado en la base que da `fabrica`.
///
/// `ahora` es la hora que llevan las altas de la auditoría; las horas de los
/// ingresos son las reales de Lattis.
pub async fn importar<F, G>(
    fabrica: &F,
    ids: &G,
    ahora: DateTime<Utc>,
    datos: &Datos,
) -> Result<Resumen, ErrorImportacion>
where
    F: FabricaUnidadDeTrabajo,
    G: GeneradorIds,
{
    let cargador = Cargador {
        fabrica,
        ids,
        ahora,
        sesion: Sesion::nueva(OperadorId::desde_uuid(OPERADOR_IMPORTADOR)),
    };
    let mut resumen = Resumen::default();
    let mut abiertos = Abiertos::default();

    let (empresas, tabla) = cargador.empresas(datos.empresas.as_deref()).await?;
    resumen.tablas.push(tabla);
    resumen
        .tablas
        .push(cargador.gafetes(datos.gafetes.as_deref()).await?);
    let (proveedoras, tabla) = cargador
        .empresas_proveedoras(datos.empresas_proveedor.as_deref())
        .await?;
    resumen.tablas.push(tabla);
    let (personal, tabla) = cargador.personal_kof(datos.personal_kof.as_deref()).await?;
    resumen.tablas.push(tabla);
    let (contratistas, tabla) = cargador
        .contratistas(datos.contratistas.as_deref(), &empresas)
        .await?;
    resumen.tablas.push(tabla);

    let mut ultimo = None;
    let (instante, tabla) = cargador
        .ingresos_contratista(
            datos.ingresos_contratista.as_deref(),
            &contratistas,
            &mut abiertos,
        )
        .await?;
    ultimo = ultimo.max(instante);
    resumen.tablas.push(tabla);
    let (instante, tabla) = cargador
        .ingresos_proveedor(
            datos.ingresos_proveedor.as_deref(),
            &proveedoras,
            &mut abiertos,
        )
        .await?;
    ultimo = ultimo.max(instante);
    resumen.tablas.push(tabla);
    let (instante, tabla) = cargador
        .ingresos_correo(datos.ingresos_correo.as_deref(), &mut abiertos)
        .await?;
    ultimo = ultimo.max(instante);
    resumen.tablas.push(tabla);
    let (instante, tabla) = cargador
        .prestamos_kof(datos.prestamos_kof.as_deref(), &personal, &mut abiertos)
        .await?;
    ultimo = ultimo.max(instante);
    resumen.tablas.push(tabla);

    cargador.fijar_reloj(ultimo).await?;
    Ok(resumen)
}
