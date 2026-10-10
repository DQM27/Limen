//! Raíz de composición de Limen: el único lugar que conecta los casos de uso
//! con los adaptadores.
//!
//! No guarda conexiones, no abre transacciones y no tiene reglas: sólo crea
//! cada caso de uso con las piezas que necesita (la base de datos, el reloj
//! y el generador de IDs). Reemplaza al `AppCore` de Lattis, que concentraba
//! 130 métodos y un candado global.
//!
//! [`Aplicacion`] es genérica: con [`AplicacionLimen`] queda armada con los
//! adaptadores reales (`SurrealDB`, la hora de Costa Rica y UUID v7), y las
//! pruebas la arman con los dobles en memoria y un reloj fijo.
//!
//! Se comparte entre pantallas y comandos sin candado (por ejemplo dentro de
//! un `Arc`): cada caso de uso crea su propia Unit of Work en cada llamada y
//! la concurrencia la maneja la base de datos.

use std::fmt;
use std::path::PathBuf;

use limen_aplicacion::casos_de_uso::consultas::{
    BuscarContratistas, BuscarEmpresas, BuscarEmpresasProveedoras, BuscarPersonalKof,
    HistorialDeCambios, ListarContratistas, ListarGafetes, PraindPorVencer, QuienesEstanAdentro,
};
use limen_aplicacion::casos_de_uso::contratistas::{
    ConsultarContratista, EditarContratista, RegistrarContratista,
};
use limen_aplicacion::casos_de_uso::correo::{RegistrarEntradaCorreo, RegistrarSalidaCorreo};
use limen_aplicacion::casos_de_uso::empresas::{RegistrarEmpresa, RenombrarEmpresa};
use limen_aplicacion::casos_de_uso::gafetes::{CambiarGafete, RegistrarGafetes};
use limen_aplicacion::casos_de_uso::ingresos::{RegistrarEntrada, RegistrarSalida};
use limen_aplicacion::casos_de_uso::kof::{
    DevolverGafeteKof, EditarPersonalKof, EntregarGafeteKof, RegistrarPersonalKof,
};
use limen_aplicacion::casos_de_uso::proveedores::{
    RegistrarEmpresaProveedora, RegistrarEntradaProveedor, RegistrarSalidaProveedor,
    RenombrarEmpresaProveedora,
};
use limen_aplicacion::puertos::{
    Consultas, ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds, Reloj,
};
use limen_infra_plataforma::{IdsV7, RelojCostaRica};
use limen_infra_surreal::AlmacenSurreal;

/// Con qué se abre la aplicación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Carpeta de la base de datos de este equipo.
    pub ruta_base: PathBuf,
}

/// No se pudo arrancar la aplicación.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("No se pudo abrir la base de datos: {0}")]
pub struct ErrorArranque(#[from] ErrorPersistencia);

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
    /// Ingreso y salida de contratistas.
    Ingresos {
        entrada: RegistrarEntrada<F, R, G>,
        salida: RegistrarSalida<F, R, G>,
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

/// Todo lo que la aplicación sabe hacer, agrupado por módulo.
///
/// `F` es la base de datos, `R` el reloj y `G` el generador de IDs.
pub struct Aplicacion<F, R, G> {
    pub empresas: Empresas<F, R, G>,
    pub contratistas: Contratistas<F, R, G>,
    pub gafetes: Gafetes<F, R, G>,
    pub ingresos: Ingresos<F, R, G>,
    pub proveedores: Proveedores<F, R, G>,
    pub correo: Correo<F, R, G>,
    pub kof: Kof<F, R, G>,
    /// Quién está adentro ahora.
    pub quienes_estan_adentro: QuienesEstanAdentro<F>,
    /// Qué cambió en un registro, quién y cuándo.
    pub historial: HistorialDeCambios<F>,
}

impl<F, R, G> fmt::Debug for Aplicacion<F, R, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Aplicacion").finish_non_exhaustive()
    }
}

impl<F, R, G> Aplicacion<F, R, G>
where
    F: FabricaUnidadDeTrabajo + Consultas + Clone,
    R: Reloj + Clone,
    G: GeneradorIds + Clone,
{
    /// Conecta todos los casos de uso con las piezas dadas.
    pub fn nueva(almacen: &F, reloj: &R, ids: &G) -> Self {
        let a = || almacen.clone();
        let r = || reloj.clone();
        let i = || ids.clone();
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
            quienes_estan_adentro: QuienesEstanAdentro::new(a()),
            historial: HistorialDeCambios::new(a()),
        }
    }
}

/// La aplicación con los adaptadores reales.
pub type AplicacionLimen = Aplicacion<AlmacenSurreal, RelojCostaRica, IdsV7>;

impl AplicacionLimen {
    /// Abre (o crea) la base de este equipo y arma la aplicación.
    pub async fn abrir(config: &Config) -> Result<Self, ErrorArranque> {
        let almacen = AlmacenSurreal::en_disco(&config.ruta_base).await?;
        Ok(Self::nueva(&almacen, &RelojCostaRica, &IdsV7))
    }
}
