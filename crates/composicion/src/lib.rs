//! Raíz de composición de Limen: el único lugar que conecta los casos de uso
//! con los adaptadores.
//!
//! No guarda conexiones, no abre transacciones y no tiene reglas: sólo crea
//! cada caso de uso con las piezas que necesita (la base de datos, el reloj
//! y el generador de IDs). Reemplaza al `AppCore` de Lattis, que concentraba
//! 130 métodos y un candado global.
//!
//! [`Aplicacion`] es genérica: con [`AplicacionLimen`] queda armada con los
//! adaptadores reales (`SurrealDB`, el reloj confiable, UUID v7 y
//! Argon2id), y las pruebas la arman con los dobles en memoria y un reloj
//! fijo.
//!
//! Se comparte entre pantallas y comandos sin candado (por ejemplo dentro de
//! un `Arc`): cada caso de uso crea su propia Unit of Work en cada llamada y
//! la concurrencia la maneja la base de datos.

#[cfg(debug_assertions)]
pub mod semilla;

use std::fmt;
use std::path::PathBuf;

use limen_aplicacion::casos_de_uso::consultas::{
    AtajosDeFecha, BuscarContratistas, BuscarEmpresas, BuscarEmpresasProveedoras,
    BuscarPersonalKof, HistorialDeCambios, ListarContratistas, ListarGafetes, ListarHistorial,
    PraindPorVencer, QuienesEstanAdentro,
};
use limen_aplicacion::casos_de_uso::contratistas::{
    ConsultarContratista, EditarContratista, RegistrarContratista, ValidarContratista,
};
use limen_aplicacion::casos_de_uso::correo::{
    RegistrarEntradaCorreo, RegistrarSalidaCorreo, ValidarEntradaCorreo,
};
use limen_aplicacion::casos_de_uso::empresas::{RegistrarEmpresa, RenombrarEmpresa};
use limen_aplicacion::casos_de_uso::gafetes::{CambiarGafete, RegistrarGafetes};
use limen_aplicacion::casos_de_uso::ingresos::{
    PrepararIngreso, RegistrarEntrada, RegistrarSalida,
};
use limen_aplicacion::casos_de_uso::kof::{
    DevolverGafeteKof, EditarPersonalKof, EntregarGafeteKof, RegistrarPersonalKof,
    ValidarEntregaKof, ValidarPersonalKof,
};
use limen_aplicacion::casos_de_uso::proveedores::{
    RegistrarEmpresaProveedora, RegistrarEntradaProveedor, RegistrarSalidaProveedor,
    RenombrarEmpresaProveedora, ValidarEntradaProveedor,
};
use limen_aplicacion::casos_de_uso::usuarios::{CambiarClave, IniciarSesion};
use limen_aplicacion::puertos::{
    Claves, Consultas, ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds, Reloj,
};
use limen_aplicacion::sesion::Origen;
use limen_infra_nube::{ConfigNube, instalar_cifrado, sincronizar};
use limen_infra_plataforma::{
    ClavesArgon2, IdsV7, RelojConfiable, SERVIDORES_NTP, leer_desfase, origen_del_equipo,
};
use limen_infra_surreal::AlmacenSurreal;

/// Con qué se abre la aplicación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Carpeta de la base de datos de este equipo.
    pub ruta_base: PathBuf,
    /// Archivo con el último desfase medido del reloj del equipo (E5), para
    /// arrancar ya corregido.
    pub ruta_desfase_reloj: PathBuf,
    /// Archivo con la identidad del equipo: su unidad operativa y su ID
    /// (M4). Si no existe, se crea con [`UNIDAD_POR_OMISION`].
    pub ruta_equipo: PathBuf,
}

/// La unidad operativa de un equipo que arranca por primera vez. Por ahora
/// hay una sola; con el bloque M la unidad llegará al vincular el equipo.
pub const UNIDAD_POR_OMISION: &str = "BRISAS";

/// No se pudo arrancar la aplicación.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorArranque {
    #[error("No se pudo abrir la base de datos: {0}")]
    Base(#[from] ErrorPersistencia),
    #[error("No se pudo preparar el cifrado de claves: {0}")]
    Claves(String),
    #[error("No se pudo leer la identidad del equipo: {0}")]
    Equipo(String),
    /// Sólo al desarrollar (ver [`semilla`]).
    #[error("No se pudo sembrar la base de desarrollo: {0}")]
    Semilla(String),
}

macro_rules! grupo {
    ($(#[$meta:meta])* $nombre:ident { $($campo:ident : $tipo:ty),+ $(,)? }) => {
        $(#[$meta])*
        pub struct $nombre<F, R, G> {
            $(pub $campo: $tipo,)+
        }

        impl<F, R, G> fmt::Debug for $nombre<F, R, G> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($nombre)).finish_non_exhaustive()
            }
        }
    };
}

grupo! {
    /// Empresas de contratistas.
    Empresas {
        registrar: RegistrarEmpresa<F, R, G>,
        renombrar: RenombrarEmpresa<F, R, G>,
        buscar: BuscarEmpresas<F>,
    }
}

grupo! {
    /// Contratistas.
    Contratistas {
        registrar: RegistrarContratista<F, R, G>,
        editar: EditarContratista<F, R, G>,
        validar: ValidarContratista<F, R>,
        consultar: ConsultarContratista<F, R>,
        buscar: BuscarContratistas<F>,
        praind_por_vencer: PraindPorVencer<F, R>,
        listar: ListarContratistas<F, R>,
    }
}

grupo! {
    /// Catálogo de gafetes.
    Gafetes {
        registrar: RegistrarGafetes<F, R, G>,
        cambiar: CambiarGafete<F, R, G>,
        listar: ListarGafetes<F>,
    }
}

grupo! {
    /// Ingreso y salida de contratistas, y el buscador y la ficha antes de
    /// registrar la entrada (`preparar`).
    Ingresos {
        entrada: RegistrarEntrada<F, R, G>,
        salida: RegistrarSalida<F, R, G>,
        preparar: PrepararIngreso<F, R>,
    }
}

grupo! {
    /// Proveedores: su catálogo de empresas y sus ingresos.
    Proveedores {
        registrar_empresa: RegistrarEmpresaProveedora<F, R, G>,
        renombrar_empresa: RenombrarEmpresaProveedora<F, R, G>,
        buscar_empresas: BuscarEmpresasProveedoras<F>,
        validar_entrada: ValidarEntradaProveedor<F>,
        entrada: RegistrarEntradaProveedor<F, R, G>,
        salida: RegistrarSalidaProveedor<F, R, G>,
    }
}

grupo! {
    /// Ingreso por correo.
    Correo {
        validar_entrada: ValidarEntradaCorreo<F>,
        entrada: RegistrarEntradaCorreo<F, R, G>,
        salida: RegistrarSalidaCorreo<F, R, G>,
    }
}

grupo! {
    /// Personal KOF y su gafete provisional.
    Kof {
        validar_personal: ValidarPersonalKof<F>,
        registrar: RegistrarPersonalKof<F, R, G>,
        editar: EditarPersonalKof<F, R, G>,
        buscar: BuscarPersonalKof<F>,
        validar_entrega: ValidarEntregaKof<F>,
        entregar_gafete: EntregarGafeteKof<F, R, G>,
        devolver_gafete: DevolverGafeteKof<F, R, G>,
    }
}

/// Inicio de sesión y cambio de la propia clave (bloque L). Es el único
/// grupo que cifra claves, por eso lleva además el cifrador `C`. Los
/// usuarios no se crean ni se administran en el equipo: vienen de la nube.
pub struct Usuarios<F, R, G, C> {
    pub iniciar_sesion: IniciarSesion<F, R, C>,
    pub cambiar_clave: CambiarClave<F, R, G, C>,
}

impl<F, R, G, C> fmt::Debug for Usuarios<F, R, G, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Usuarios").finish_non_exhaustive()
    }
}

/// Todo lo que la aplicación sabe hacer, agrupado por módulo.
///
/// `F` es la base de datos, `R` el reloj, `G` el generador de IDs y `C` el
/// cifrador de claves (Argon2id si no se dice otro).
pub struct Aplicacion<F, R, G, C = ClavesArgon2> {
    pub empresas: Empresas<F, R, G>,
    pub contratistas: Contratistas<F, R, G>,
    pub gafetes: Gafetes<F, R, G>,
    pub ingresos: Ingresos<F, R, G>,
    pub proveedores: Proveedores<F, R, G>,
    pub correo: Correo<F, R, G>,
    pub kof: Kof<F, R, G>,
    pub usuarios: Usuarios<F, R, G, C>,
    /// Quién está adentro ahora.
    pub quienes_estan_adentro: QuienesEstanAdentro<F, R>,
    /// Qué cambió en un registro, quién y cuándo.
    pub historial: HistorialDeCambios<F>,
    /// Los ingresos y salidas de las cuatro vías en un rango de fechas.
    pub historial_de_ingresos: ListarHistorial<F, R>,
    /// Los accesos rápidos de fecha (hoy, esta semana…) del historial.
    pub atajos_de_fecha: AtajosDeFecha<R>,
}

impl<F, R, G, C> fmt::Debug for Aplicacion<F, R, G, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Aplicacion").finish_non_exhaustive()
    }
}

impl<F, R, G, C> Aplicacion<F, R, G, C>
where
    F: FabricaUnidadDeTrabajo + Consultas + Clone,
    R: Reloj + Clone,
    G: GeneradorIds + Clone,
    C: Claves + Clone,
{
    /// Conecta todos los casos de uso con las piezas dadas.
    /// `origen` es dónde está este equipo: la unidad operativa y su ID. Va
    /// en cada sesión que se abre aquí y, con ella, en cada hecho (M4).
    pub fn nueva(almacen: &F, reloj: &R, ids: &G, claves: &C, origen: &Origen) -> Self {
        let a = || almacen.clone();
        let r = || reloj.clone();
        let i = || ids.clone();
        let c = || claves.clone();
        Self {
            empresas: Empresas {
                registrar: RegistrarEmpresa::new(a(), r(), i()),
                renombrar: RenombrarEmpresa::new(a(), r(), i()),
                buscar: BuscarEmpresas::new(a()),
            },
            contratistas: Contratistas {
                registrar: RegistrarContratista::new(a(), r(), i()),
                editar: EditarContratista::new(a(), r(), i()),
                validar: ValidarContratista::new(a(), r()),
                consultar: ConsultarContratista::new(a(), r()),
                buscar: BuscarContratistas::new(a()),
                praind_por_vencer: PraindPorVencer::new(a(), r()),
                listar: ListarContratistas::new(a(), r()),
            },
            gafetes: Gafetes {
                registrar: RegistrarGafetes::new(a(), r(), i()),
                cambiar: CambiarGafete::new(a(), r(), i()),
                listar: ListarGafetes::new(a()),
            },
            ingresos: Ingresos {
                entrada: RegistrarEntrada::new(a(), r(), i()),
                salida: RegistrarSalida::new(a(), r(), i()),
                preparar: PrepararIngreso::new(a(), r()),
            },
            proveedores: Proveedores {
                registrar_empresa: RegistrarEmpresaProveedora::new(a(), r(), i()),
                renombrar_empresa: RenombrarEmpresaProveedora::new(a(), r(), i()),
                buscar_empresas: BuscarEmpresasProveedoras::new(a()),
                validar_entrada: ValidarEntradaProveedor::new(a()),
                entrada: RegistrarEntradaProveedor::new(a(), r(), i()),
                salida: RegistrarSalidaProveedor::new(a(), r(), i()),
            },
            correo: Correo {
                validar_entrada: ValidarEntradaCorreo::new(a()),
                entrada: RegistrarEntradaCorreo::new(a(), r(), i()),
                salida: RegistrarSalidaCorreo::new(a(), r(), i()),
            },
            kof: Kof {
                validar_personal: ValidarPersonalKof::new(a()),
                registrar: RegistrarPersonalKof::new(a(), r(), i()),
                editar: EditarPersonalKof::new(a(), r(), i()),
                buscar: BuscarPersonalKof::new(a()),
                validar_entrega: ValidarEntregaKof::new(a()),
                entregar_gafete: EntregarGafeteKof::new(a(), r(), i()),
                devolver_gafete: DevolverGafeteKof::new(a(), r(), i()),
            },
            usuarios: Usuarios {
                iniciar_sesion: IniciarSesion::new(a(), r(), c(), origen.clone()),
                cambiar_clave: CambiarClave::new(a(), r(), i(), c()),
            },
            quienes_estan_adentro: QuienesEstanAdentro::new(a(), r()),
            historial: HistorialDeCambios::new(a()),
            historial_de_ingresos: ListarHistorial::new(a(), r()),
            atajos_de_fecha: AtajosDeFecha::new(r()),
        }
    }
}

/// La aplicación con los adaptadores reales.
/// Los adaptadores reales, con nombre propio: así una app nombra sus tipos
/// (por ejemplo, el estado de Tauri) sin depender de la infraestructura.
pub type Almacen = AlmacenSurreal;
pub type RelojLimen = RelojConfiable;
pub type IdsLimen = IdsV7;
pub type ClavesLimen = ClavesArgon2;

pub type AplicacionLimen = Aplicacion<Almacen, RelojLimen, IdsLimen, ClavesLimen>;

impl AplicacionLimen {
    /// Arranca la aplicación de este equipo: crea el reloj confiable con el
    /// último desfase guardado, lo corrige contra NTP en segundo plano (E5:
    /// hasta la primera medición la hora sale sin comprobar y queda marcada)
    /// y abre la base.
    ///
    /// Si el equipo tiene configurada la nube (variables `LIMEN_NUBE_*`, ver
    /// `docs/nube.md`), lanza además la sincronización en segundo plano: lo
    /// que se registra sube a la nube a los pocos segundos, y sin conexión se
    /// acumula y sube al volver. Sin esa configuración la app trabaja igual,
    /// sólo en el equipo. Hay que llamarlo dentro del runtime de `tokio`.
    pub async fn arrancar(config: &Config) -> Result<Self, ErrorArranque> {
        let reloj = RelojConfiable::new(leer_desfase(&config.ruta_desfase_reloj));
        reloj.sincronizar_en_segundo_plano(&SERVIDORES_NTP, config.ruta_desfase_reloj.clone());
        let (aplicacion, almacen) = Self::abrir_con_almacen(config, &reloj).await?;
        match ConfigNube::desde_entorno(|nombre| std::env::var(nombre).ok()) {
            Ok(nube) => {
                instalar_cifrado();
                tokio::spawn(sincronizar(almacen, nube));
            }
            Err(motivo) => log::info!("sin sincronización con la nube: {motivo}"),
        }
        Ok(aplicacion)
    }

    /// Abre (o crea) la base de este equipo y arma la aplicación con este
    /// reloj, sin sincronizarlo (las pruebas no salen a la red). La identidad
    /// del equipo (unidad operativa e ID) sale de `config.ruta_equipo`.
    pub async fn abrir(config: &Config, reloj: &RelojConfiable) -> Result<Self, ErrorArranque> {
        Ok(Self::abrir_con_almacen(config, reloj).await?.0)
    }

    async fn abrir_con_almacen(
        config: &Config,
        reloj: &RelojConfiable,
    ) -> Result<(Self, AlmacenSurreal), ErrorArranque> {
        let origen = origen_del_equipo(&config.ruta_equipo, UNIDAD_POR_OMISION)
            .map_err(ErrorArranque::Equipo)?;
        let almacen = AlmacenSurreal::en_disco(&config.ruta_base, origen.clone()).await?;
        let claves = ClavesArgon2::new().map_err(ErrorArranque::Claves)?;
        // Sólo al desarrollar: la versión para producción no trae la semilla.
        #[cfg(debug_assertions)]
        semilla::sembrar_usuarios(&almacen, &claves)
            .await
            .map_err(ErrorArranque::Semilla)?;
        let aplicacion = Self::nueva(&almacen, reloj, &IdsV7, &claves, &origen);
        Ok((aplicacion, almacen))
    }
}
