//! Conexión de Limen con la nube: la instancia de `SurrealDB` Cloud.
//!
//! Por ahora hace tres cosas: **conectar** (WebSocket seguro, `wss://`, con
//! usuario y clave), **subir** la base de un equipo (`subida`) y **aplicar el
//! esquema**, que es el mismo de los equipos
//! (`limen_infra_surreal::ESQUEMA`): así la nube y los equipos no pueden
//! quedar distintos. La sincronización de hechos (sección 9.2 de
//! `docs/arquitectura.md`) se construye encima de esto.
//!
//! La configuración sale de variables de entorno ([`ConfigNube::desde_entorno`]),
//! nunca del código: el repositorio es público.

mod bajada;
mod en_vivo;
mod espera;
mod presencia;
mod sincronizacion;
mod subida;
mod vinculacion;

use std::fmt;

use limen_infra_surreal::{BASE, ESQUEMA, NAMESPACE};
use surrealdb::Surreal;
use surrealdb::engine::any::{Any, connect};
use surrealdb::opt::auth::{Database, Namespace, Root};

pub use en_vivo::Escucha;
pub use presencia::LATIDO;
pub use sincronizacion::{FuenteDeConfig, RED_DE_SEGURIDAD, REINTENTO, config_fija, sincronizar};
pub use subida::{Copia, TABLAS};
pub use vinculacion::{
    ARCHIVO_CLAVE, ARCHIVO_VINCULACION, EquipoVinculado, ErrorDeVinculacion, PANEL_POR_OMISION,
    VAR_PANEL, esta_vinculado, pedir_acceso, vincular,
};

/// Elige el proveedor de cifrado de las conexiones seguras (`ring`). Con
/// `ring` y `aws-lc-rs` a la vez en el árbol de dependencias, `rustls` no
/// elige solo y la primera conexión `wss://` fallaría. Hay que llamarlo una
/// vez al arrancar, antes de conectar; llamarlo de nuevo no hace nada.
pub fn instalar_cifrado() {
    // Si ya hay uno instalado, el error sólo avisa de eso: se ignora.
    drop(rustls::crypto::ring::default_provider().install_default());
}

/// La dirección de la instancia, por ejemplo
/// `wss://limen-xxxx.aws-use1.surreal.cloud`.
pub const VAR_URL: &str = "LIMEN_NUBE_URL";
/// Usuario y clave con que se entra a la instancia.
pub const VAR_USUARIO: &str = "LIMEN_NUBE_USUARIO";
pub const VAR_CLAVE: &str = "LIMEN_NUBE_CLAVE";
/// A qué nivel pertenece ese usuario: `root` (por omisión), `namespace` o
/// `base`.
pub const VAR_NIVEL: &str = "LIMEN_NUBE_NIVEL";

/// A qué nivel de `SurrealDB` pertenece el usuario. Sólo `root` y
/// `namespace` pueden crear la base si todavía no existe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nivel {
    Root,
    Namespace,
    Base,
    /// Un token de acceso que el Worker del panel emitió a este equipo (vinculado con su
    /// código). La clave de las `Credenciales` es el token; no hay usuario ni clave de base.
    Token,
}

impl Nivel {
    fn desde_texto(texto: &str) -> Option<Self> {
        match texto.trim().to_lowercase().as_str() {
            "root" => Some(Self::Root),
            "namespace" | "ns" => Some(Self::Namespace),
            "base" | "database" | "db" => Some(Self::Base),
            _ => None,
        }
    }
}

/// Usuario y clave. La clave nunca aparece en `Debug` ni en los errores.
#[derive(Clone, PartialEq, Eq)]
pub struct Credenciales {
    pub nivel: Nivel,
    pub usuario: String,
    pub clave: String,
}

impl Credenciales {
    /// El acceso de un equipo vinculado: el token que le dio el Worker.
    pub fn con_token(token: String) -> Self {
        Self {
            nivel: Nivel::Token,
            usuario: "equipo".to_owned(),
            clave: token,
        }
    }
}

impl fmt::Debug for Credenciales {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credenciales")
            .field("nivel", &self.nivel)
            .field("usuario", &self.usuario)
            .field("clave", &"***")
            .finish()
    }
}

/// Con qué se conecta a la nube.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigNube {
    pub url: String,
    /// `None` sólo para una base local sin usuarios (las pruebas, `mem://`).
    pub credenciales: Option<Credenciales>,
}

/// Algo salió mal al hablar con la nube.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorNube {
    #[error("Falta la variable de entorno {0}")]
    FaltaVariable(&'static str),
    #[error("{VAR_NIVEL} no es válido: use root, namespace o base")]
    NivelInvalido,
    #[error("No se pudo conectar con la nube: {0}")]
    Conexion(String),
    #[error("La nube no aceptó el usuario y la clave: {0}")]
    Autenticacion(String),
    #[error("No se pudo aplicar el esquema en la nube: {0}")]
    Esquema(String),
    #[error("No se pudo consultar la nube: {0}")]
    Consulta(String),
    #[error("No se pudo subir a la nube: {0}")]
    Subida(String),
    #[error("No se pudo bajar de la nube: {0}")]
    Bajada(String),
}

impl ConfigNube {
    /// Lee la configuración con `leer` (en la app, `std::env::var`). Usuario
    /// y clave son obligatorios: la nube nunca se abre sin ellos.
    pub fn desde_entorno(leer: impl Fn(&str) -> Option<String>) -> Result<Self, ErrorNube> {
        let obligatoria = |nombre: &'static str| {
            leer(nombre)
                .map(|valor| valor.trim().to_owned())
                .filter(|valor| !valor.is_empty())
                .ok_or(ErrorNube::FaltaVariable(nombre))
        };
        let nivel = match leer(VAR_NIVEL) {
            Some(texto) if !texto.trim().is_empty() => {
                Nivel::desde_texto(&texto).ok_or(ErrorNube::NivelInvalido)?
            }
            _ => Nivel::Root,
        };
        Ok(Self {
            url: obligatoria(VAR_URL)?,
            credenciales: Some(Credenciales {
                nivel,
                usuario: obligatoria(VAR_USUARIO)?,
                // La clave no se recorta: un espacio puede ser parte de ella.
                clave: leer(VAR_CLAVE)
                    .filter(|clave| !clave.is_empty())
                    .ok_or(ErrorNube::FaltaVariable(VAR_CLAVE))?,
            }),
        })
    }
}

/// Lo que se sabe de la nube después de conectar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstadoNube {
    /// Versión de `SurrealDB` de la instancia.
    pub version: String,
    /// Las tablas de la base de Limen, en orden alfabético.
    pub tablas: Vec<String>,
}

/// Una conexión abierta con la nube, ya dentro del namespace y la base de
/// Limen.
#[derive(Debug, Clone)]
pub struct Nube {
    db: Surreal<Any>,
}

impl Nube {
    /// Conecta, entra con las credenciales y se ubica en la base de Limen.
    /// Si el usuario puede (root o namespace), crea el namespace y la base
    /// cuando todavía no existen.
    pub async fn conectar(config: &ConfigNube) -> Result<Self, ErrorNube> {
        let db = connect(config.url.as_str())
            .await
            .map_err(|e| ErrorNube::Conexion(e.to_string()))?;
        if let Some(credenciales) = &config.credenciales {
            entrar(&db, credenciales).await?;
            crear_si_falta(&db, credenciales.nivel).await?;
        }
        db.use_ns(NAMESPACE)
            .use_db(BASE)
            .await
            .map_err(|e| ErrorNube::Conexion(e.to_string()))?;
        Ok(Self { db })
    }

    /// Aplica el esquema completo de Limen. Es declarativo (`OVERWRITE`):
    /// aplicarlo otra vez no cambia los datos, sólo deja el esquema al día.
    pub async fn aplicar_esquema(&self) -> Result<(), ErrorNube> {
        self.db
            .query(ESQUEMA)
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map(drop)
            .map_err(|e| ErrorNube::Esquema(e.to_string()))
    }

    /// La versión de la instancia y las tablas que tiene la base de Limen.
    pub async fn estado(&self) -> Result<EstadoNube, ErrorNube> {
        let consulta = |e: surrealdb::Error| ErrorNube::Consulta(e.to_string());
        let version = self.db.version().await.map_err(consulta)?.to_string();
        let mut tablas: Vec<String> = self
            .db
            .query("RETURN object::keys((INFO FOR DB).tables)")
            .await
            .map_err(consulta)?
            .take(0)
            .map_err(consulta)?;
        tablas.sort();
        Ok(EstadoNube { version, tablas })
    }
}

async fn entrar(db: &Surreal<Any>, credenciales: &Credenciales) -> Result<(), ErrorNube> {
    let usuario = credenciales.usuario.clone();
    let clave = credenciales.clave.clone();
    let resultado = match credenciales.nivel {
        Nivel::Root => db
            .signin(Root {
                username: usuario,
                password: clave,
            })
            .await
            .map(drop),
        Nivel::Namespace => db
            .signin(Namespace {
                namespace: NAMESPACE.to_owned(),
                username: usuario,
                password: clave,
            })
            .await
            .map(drop),
        Nivel::Base => db
            .signin(Database {
                namespace: NAMESPACE.to_owned(),
                database: BASE.to_owned(),
                username: usuario,
                password: clave,
            })
            .await
            .map(drop),
        Nivel::Token => db.authenticate(clave).await.map(drop),
    };
    resultado.map_err(|e| ErrorNube::Autenticacion(e.to_string()))
}

/// Crea el namespace y la base de Limen si faltan, según lo que el usuario
/// puede hacer. Un usuario de base no puede: la base ya tiene que existir.
async fn crear_si_falta(db: &Surreal<Any>, nivel: Nivel) -> Result<(), ErrorNube> {
    let sentencias = match nivel {
        Nivel::Root => format!(
            "DEFINE NAMESPACE IF NOT EXISTS {NAMESPACE}; USE NS {NAMESPACE}; \
             DEFINE DATABASE IF NOT EXISTS {BASE};"
        ),
        Nivel::Namespace => format!("USE NS {NAMESPACE}; DEFINE DATABASE IF NOT EXISTS {BASE};"),
        Nivel::Base | Nivel::Token => return Ok(()),
    };
    db.query(sentencias)
        .await
        .and_then(surrealdb::IndexedResults::check)
        .map(drop)
        .map_err(|e| ErrorNube::Esquema(e.to_string()))
}
