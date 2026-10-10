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

use std::fmt;
use std::path::PathBuf;

use limen_aplicacion::casos_de_uso::consultas::{
    AtajosDeFecha, BuscarContratistas, BuscarEmpresas, BuscarEmpresasProveedoras,
    BuscarPersonalKof, HistorialDeCambios, ListarContratistas, ListarGafetes, ListarHistorial,
    PraindPorVencer, QuienesEstanAdentro,
};
use limen_aplicacion::casos_de_uso::contratistas::{
    ConsultarContratista, EditarContratista, RegistrarContratista,
};
use limen_aplicacion::casos_de_uso::correo::{RegistrarEntradaCorreo, RegistrarSalidaCorreo};
use limen_aplicacion::casos_de_uso::empresas::{RegistrarEmpresa, RenombrarEmpresa};
use limen_aplicacion::casos_de_uso::gafetes::{CambiarGafete, RegistrarGafetes};
use limen_aplicacion::casos_de_uso::ingresos::{
    PrepararIngreso, RegistrarEntrada, RegistrarSalida,
};
use limen_aplicacion::casos_de_uso::kof::{
    DevolverGafeteKof, EditarPersonalKof, EntregarGafeteKof, RegistrarPersonalKof,
};
use limen_aplicacion::casos_de_uso::proveedores::{
    RegistrarEmpresaProveedora, RegistrarEntradaProveedor, RegistrarSalidaProveedor,
    RenombrarEmpresaProveedora,
};
use limen_aplicacion::casos_de_uso::usuarios::{
    CambiarContrasena, CrearPrimerUsuario, EditarUsuario, HayUsuarios, IniciarSesion,
    ListarUsuarios, RegistrarUsuario, RestablecerContrasena,
};
use limen_aplicacion::puertos::{
    Consultas, Contrasenas, ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds, Reloj,
};
use limen_infra_plataforma::{ContrasenasArgon2, IdsV7, RelojConfiable};
use limen_infra_surreal::AlmacenSurreal;

/// Con qué se abre la aplicación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Carpeta de la base de datos de este equipo.
    pub ruta_base: PathBuf,
}

/// No se pudo arrancar la aplicación.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorArranque {
    #[error("No se pudo abrir la base de datos: {0}")]
    Base(#[from] ErrorPersistencia),
    #[error("No se pudo preparar el cifrado de contraseñas: {0}")]
    Contrasenas(String),
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
        entrada: RegistrarEntradaProveedor<F, R, G>,
        salida: RegistrarSalidaProveedor<F, R, G>,
    }
}

grupo! {
    /// Ingreso por correo.
    Correo {
        entrada: RegistrarEntradaCorreo<F, R, G>,
        salida: RegistrarSalidaCorreo<F, R, G>,
    }
}

grupo! {
    /// Personal KOF y su gafete provisional.
    Kof {
        registrar: RegistrarPersonalKof<F, R, G>,
        editar: EditarPersonalKof<F, R, G>,
        buscar: BuscarPersonalKof<F>,
        entregar_gafete: EntregarGafeteKof<F, R, G>,
        devolver_gafete: DevolverGafeteKof<F, R, G>,
    }
}

/// Usuarios e inicio de sesión (bloque L). Es el único grupo que cifra
/// contraseñas, por eso lleva además el cifrador `C`.
pub struct Usuarios<F, R, G, C> {
    pub hay_usuarios: HayUsuarios<F>,
    pub crear_primero: CrearPrimerUsuario<F, R, G, C>,
    pub iniciar_sesion: IniciarSesion<F, R, C>,
    pub registrar: RegistrarUsuario<F, R, G, C>,
    pub editar: EditarUsuario<F, R, G>,
    pub cambiar_contrasena: CambiarContrasena<F, R, G, C>,
    pub restablecer_contrasena: RestablecerContrasena<F, R, G, C>,
    pub listar: ListarUsuarios<F>,
}

impl<F, R, G, C> fmt::Debug for Usuarios<F, R, G, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Usuarios").finish_non_exhaustive()
    }
}

/// Todo lo que la aplicación sabe hacer, agrupado por módulo.
///
/// `F` es la base de datos, `R` el reloj, `G` el generador de IDs y `C` el
/// cifrador de contraseñas (Argon2id si no se dice otro).
pub struct Aplicacion<F, R, G, C = ContrasenasArgon2> {
    pub empresas: Empresas<F, R, G>,
    pub contratistas: Contratistas<F, R, G>,
    pub gafetes: Gafetes<F, R, G>,
    pub ingresos: Ingresos<F, R, G>,
    pub proveedores: Proveedores<F, R, G>,
    pub correo: Correo<F, R, G>,
    pub kof: Kof<F, R, G>,
    pub usuarios: Usuarios<F, R, G, C>,
    /// Quién está adentro ahora.
    pub quienes_estan_adentro: QuienesEstanAdentro<F>,
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
    C: Contrasenas + Clone,
{
    /// Conecta todos los casos de uso con las piezas dadas.
    pub fn nueva(almacen: &F, reloj: &R, ids: &G, contrasenas: &C) -> Self {
        let a = || almacen.clone();
        let r = || reloj.clone();
        let i = || ids.clone();
        let c = || contrasenas.clone();
        Self {
            empresas: Empresas {
                registrar: RegistrarEmpresa::new(a(), r(), i()),
                renombrar: RenombrarEmpresa::new(a(), r(), i()),
                buscar: BuscarEmpresas::new(a()),
            },
            contratistas: Contratistas {
                registrar: RegistrarContratista::new(a(), r(), i()),
                editar: EditarContratista::new(a(), r(), i()),
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
                entrada: RegistrarEntradaProveedor::new(a(), r(), i()),
                salida: RegistrarSalidaProveedor::new(a(), r(), i()),
            },
            correo: Correo {
                entrada: RegistrarEntradaCorreo::new(a(), r(), i()),
                salida: RegistrarSalidaCorreo::new(a(), r(), i()),
            },
            kof: Kof {
                registrar: RegistrarPersonalKof::new(a(), r(), i()),
                editar: EditarPersonalKof::new(a(), r(), i()),
                buscar: BuscarPersonalKof::new(a()),
                entregar_gafete: EntregarGafeteKof::new(a(), r(), i()),
                devolver_gafete: DevolverGafeteKof::new(a(), r(), i()),
            },
            usuarios: Usuarios {
                hay_usuarios: HayUsuarios::new(a()),
                crear_primero: CrearPrimerUsuario::new(a(), r(), i(), c()),
                iniciar_sesion: IniciarSesion::new(a(), r(), c()),
                registrar: RegistrarUsuario::new(a(), r(), i(), c()),
                editar: EditarUsuario::new(a(), r(), i()),
                cambiar_contrasena: CambiarContrasena::new(a(), r(), i(), c()),
                restablecer_contrasena: RestablecerContrasena::new(a(), r(), i(), c()),
                listar: ListarUsuarios::new(a()),
            },
            quienes_estan_adentro: QuienesEstanAdentro::new(a()),
            historial: HistorialDeCambios::new(a()),
            historial_de_ingresos: ListarHistorial::new(a(), r()),
            atajos_de_fecha: AtajosDeFecha::new(r()),
        }
    }
}

/// La aplicación con los adaptadores reales.
pub type AplicacionLimen = Aplicacion<AlmacenSurreal, RelojConfiable, IdsV7, ContrasenasArgon2>;

impl AplicacionLimen {
    /// Abre (o crea) la base de este equipo y arma la aplicación. El reloj
    /// lo crea quien arranca la app, que también lo sincroniza (ver
    /// [`RelojConfiable::sincronizar_en_segundo_plano`]).
    pub async fn abrir(config: &Config, reloj: &RelojConfiable) -> Result<Self, ErrorArranque> {
        let almacen = AlmacenSurreal::en_disco(&config.ruta_base).await?;
        let contrasenas = ContrasenasArgon2::new().map_err(ErrorArranque::Contrasenas)?;
        Ok(Self::nueva(&almacen, reloj, &IdsV7, &contrasenas))
    }
}
