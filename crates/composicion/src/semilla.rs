//! La semilla de desarrollo: usuarios de prueba para poder entrar a la app
//! mientras no exista la nube.
//!
//! Los usuarios son globales y vienen de la nube; un equipo no los crea. Para
//! desarrollar, la base se siembra con estos usuarios al abrirla si todavía no
//! tiene ninguno. El módulo sólo existe en compilación de depuración
//! (`debug_assertions`): la versión para producción no lo trae, así que nunca
//! siembra nada. Todos los datos son inventados.

use limen_aplicacion::puertos::{
    Claves, FabricaUnidadDeTrabajo, RepositorioUsuarios, UnidadDeTrabajo,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::usuario::{ClaveNueva, Usuario, UsuarioGuardado};
use uuid::Uuid;

/// La clave de todos los usuarios de desarrollo.
pub const CLAVE: &str = "desarrollo";

/// ID, cédula y nombre de cada usuario de desarrollo. Son dos para poder
/// probar una entrada y una salida registradas por operadores distintos.
pub const USUARIOS: [(u128, &str, &str); 2] = [
    (1, "100000001", "OPERADOR UNO"),
    (2, "100000002", "OPERADOR DOS"),
];

/// El ID fijo de un usuario de desarrollo: igual en cada equipo y en cada
/// siembra.
pub fn id(n: u128) -> OperadorId {
    OperadorId::desde_uuid(Uuid::from_u128(
        0x00de_0000_0000_0000_0000_0000_0000_0000 + n,
    ))
}

/// Siembra los usuarios de desarrollo si la base no tiene ninguno. Devuelve
/// si sembró.
pub async fn sembrar_usuarios<F, C>(fabrica: &F, claves: &C) -> Result<bool, String>
where
    F: FabricaUnidadDeTrabajo,
    C: Claves,
{
    let mut uow = fabrica.nueva();
    if uow
        .usuarios()
        .hay_usuarios()
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(false);
    }
    for (n, cedula, nombre) in USUARIOS {
        let cedula = Cedula::normalizar(cedula).map_err(|e| e.to_string())?;
        let clave = ClaveNueva::nueva(CLAVE, &cedula).map_err(|e| e.to_string())?;
        uow.usuarios().guardar(&Usuario::restaurar(UsuarioGuardado {
            id: id(n),
            cedula,
            nombre: NombrePersona::nuevo(nombre).map_err(|e| e.to_string())?,
            activo: true,
            clave: claves.cifrar(&clave)?,
            debe_cambiar_clave: false,
        }));
    }
    uow.confirmar().await.map_err(|e| e.to_string())?;
    Ok(true)
}
