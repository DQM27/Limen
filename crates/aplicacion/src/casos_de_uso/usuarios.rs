//! Casos de uso de los usuarios (bloque L): el inicio de sesión, el primer
//! usuario del equipo y la administración de usuarios.
//!
//! TEMPORAL: mientras no exista el panel de la nube, cualquier operador con
//! sesión registra y edita usuarios en el equipo (L1: hay un solo rol).

use std::convert::Infallible;

use limen_dominio::auditoria::CambioCampo;
use limen_dominio::cedula::Cedula;
use limen_dominio::usuario::{
    ClaveNueva, ErrorInicioSesion, ErrorUsuario, HashClave, HechosUsuario, Usuario,
    cedula_de_usuario, decidir_inicio, sumar_fallo, verificar_bloqueo,
};

use crate::errores::ErrorCaso;
use crate::puertos::{
    AccionAuditada, Claves, EntradaAuditoria, FabricaUnidadDeTrabajo, GeneradorIds,
    RegistroAuditado, RegistroAuditoria, Reloj, RepositorioIntentosInicio, RepositorioUsuarios,
    Restriccion, UnidadDeTrabajo,
};
use crate::sesion::{OperadorId, Sesion};

pub type ErrorUsuarios = ErrorCaso<ErrorUsuario>;
pub type ErrorInicio = ErrorCaso<ErrorInicioSesion>;

/// La base rechazó la cédula porque otro equipo la guardó primero.
fn conflicto_de_cedula(restriccion: Restriccion) -> Option<ErrorUsuario> {
    (restriccion == Restriccion::CedulaUsuario).then_some(ErrorUsuario::CedulaRepetida)
}

/// Valida la clave nueva (L3) y la cifra.
fn cifrar(claves: &impl Claves, texto: &str, cedula: &Cedula) -> Result<HashClave, ErrorUsuarios> {
    let nueva = ClaveNueva::nueva(texto, cedula).map_err(ErrorCaso::Negocio)?;
    claves.cifrar(&nueva).map_err(ErrorCaso::Tecnico)
}

/// Quién acaba de entrar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SesionIniciada {
    pub sesion: Sesion,
    pub cedula: String,
    pub nombre: String,
    /// La clave la puso otra persona: la interfaz pide cambiarla antes
    /// de seguir.
    pub debe_cambiar_clave: bool,
}

impl SesionIniciada {
    fn de(usuario: &Usuario) -> Self {
        Self {
            sesion: Sesion::nueva(usuario.id()),
            cedula: usuario.cedula().to_string(),
            nombre: usuario.nombre().to_string(),
            debe_cambiar_clave: usuario.debe_cambiar_clave(),
        }
    }
}

/// Un usuario en la lista. Nunca lleva la clave ni su hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilaUsuario {
    pub id: OperadorId,
    pub cedula: String,
    pub nombre: String,
    pub activo: bool,
    pub debe_cambiar_clave: bool,
}

// --- Inicio de sesión ---

#[derive(Debug)]
pub struct IniciarSesion<F, R, C> {
    fabrica: F,
    reloj: R,
    claves: C,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, C: Claves> IniciarSesion<F, R, C> {
    pub const fn new(fabrica: F, reloj: R, claves: C) -> Self {
        Self {
            fabrica,
            reloj,
            claves,
        }
    }

    pub async fn ejecutar(&self, cedula: &str, clave: &str) -> Result<SesionIniciada, ErrorInicio> {
        let Ok(cedula) = cedula_de_usuario(cedula) else {
            // Una cédula mal escrita no puede ser de nadie, pero se tarda lo
            // mismo en contestar.
            self.claves.verificar(clave, None);
            return Err(ErrorCaso::Negocio(ErrorInicioSesion::CredencialesInvalidas));
        };
        let ahora = self.reloj.ahora();
        let mut uow = self.fabrica.nueva();
        let intentos = uow.intentos_inicio().obtener(&cedula).await?;
        verificar_bloqueo(intentos, ahora).map_err(ErrorCaso::Negocio)?;
        let usuario = uow.usuarios().obtener_por_cedula(&cedula).await?;
        let correcta = self
            .claves
            .verificar(clave, usuario.as_ref().map(Usuario::clave));

        match decidir_inicio(usuario.as_ref(), correcta) {
            Ok(usuario) => {
                let iniciada = SesionIniciada::de(usuario);
                if intentos.is_some() {
                    uow.intentos_inicio().borrar(&cedula);
                    uow.confirmar().await?;
                }
                Ok(iniciada)
            }
            Err(error) => {
                if error.suma_intento() {
                    uow.intentos_inicio()
                        .anotar(&cedula, sumar_fallo(intentos, ahora));
                    uow.confirmar().await?;
                }
                Err(ErrorCaso::Negocio(error))
            }
        }
    }
}

// --- Primer usuario ---

/// Si el equipo ya tiene usuarios. Sin ninguno, la interfaz ofrece crear el
/// primero en vez de pedir la sesión.
#[derive(Debug)]
pub struct HayUsuarios<F> {
    fabrica: F,
}

impl<F: FabricaUnidadDeTrabajo> HayUsuarios<F> {
    pub const fn new(fabrica: F) -> Self {
        Self { fabrica }
    }

    pub async fn ejecutar(&self) -> Result<bool, ErrorCaso<Infallible>> {
        let mut uow = self.fabrica.nueva();
        Ok(uow.usuarios().hay_usuarios().await?)
    }
}

/// Crea el primer usuario de un equipo recién instalado y le abre la sesión.
#[derive(Debug)]
pub struct CrearPrimerUsuario<F, R, G, C> {
    fabrica: F,
    reloj: R,
    ids: G,
    claves: C,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds, C: Claves>
    CrearPrimerUsuario<F, R, G, C>
{
    pub const fn new(fabrica: F, reloj: R, ids: G, claves: C) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
            claves,
        }
    }

    pub async fn ejecutar(
        &self,
        cedula: &str,
        nombre: &str,
        clave: &str,
    ) -> Result<SesionIniciada, ErrorUsuarios> {
        let cedula = cedula_de_usuario(cedula).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let hay_usuarios = uow.usuarios().hay_usuarios().await?;
        let hash = cifrar(&self.claves, clave, &cedula)?;
        let id = OperadorId::desde_uuid(self.ids.nuevo());
        let usuario = Usuario::crear_primero(id, cedula, nombre, hash, hay_usuarios)
            .map_err(ErrorCaso::Negocio)?;
        let iniciada = SesionIniciada::de(&usuario);

        uow.usuarios().guardar(&usuario);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            RegistroAuditado::Usuario(id),
            AccionAuditada::Alta,
            usuario.cambios_de_alta(),
            &iniciada.sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_cedula))?;
        Ok(iniciada)
    }
}

// --- Administración ---

/// Registra un usuario con una clave temporal: deberá cambiarla al
/// entrar.
#[derive(Debug)]
pub struct RegistrarUsuario<F, R, G, C> {
    fabrica: F,
    reloj: R,
    ids: G,
    claves: C,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds, C: Claves> RegistrarUsuario<F, R, G, C> {
    pub const fn new(fabrica: F, reloj: R, ids: G, claves: C) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
            claves,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        cedula: &str,
        nombre: &str,
        clave_temporal: &str,
    ) -> Result<OperadorId, ErrorUsuarios> {
        let cedula = cedula_de_usuario(cedula).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let hechos = HechosUsuario {
            cedula_en_uso: uow.usuarios().cedula_en_uso(&cedula).await?,
        };
        let hash = cifrar(&self.claves, clave_temporal, &cedula)?;
        let id = OperadorId::desde_uuid(self.ids.nuevo());
        let usuario = Usuario::registrar(id, cedula, nombre, hash, true, hechos)
            .map_err(ErrorCaso::Negocio)?;

        uow.usuarios().guardar(&usuario);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            RegistroAuditado::Usuario(id),
            AccionAuditada::Alta,
            usuario.cambios_de_alta(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_cedula))?;
        Ok(id)
    }
}

/// Cambia el nombre de un usuario o lo activa o desactiva.
#[derive(Debug)]
pub struct EditarUsuario<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> EditarUsuario<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    /// Devuelve lo que cambió (vacío si todo era igual: entonces no se
    /// escribe nada).
    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        id: OperadorId,
        nombre: &str,
        activo: bool,
    ) -> Result<Vec<CambioCampo>, ErrorUsuarios> {
        let mut uow = self.fabrica.nueva();
        let mut usuario = uow
            .usuarios()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let cambios = usuario
            .editar(nombre, activo, sesion.operador())
            .map_err(ErrorCaso::Negocio)?;
        if cambios.is_empty() {
            return Ok(cambios);
        }
        uow.usuarios().guardar(&usuario);
        anotar_edicion(
            &mut uow,
            &self.ids,
            &self.reloj,
            sesion,
            id,
            cambios.clone(),
        );
        uow.confirmar().await.map_err(ErrorCaso::from)?;
        Ok(cambios)
    }
}

fn anotar_edicion(
    uow: &mut impl UnidadDeTrabajo,
    ids: &impl GeneradorIds,
    reloj: &impl Reloj,
    sesion: &Sesion,
    id: OperadorId,
    cambios: Vec<CambioCampo>,
) {
    uow.auditoria().anotar(EntradaAuditoria::nueva(
        ids.nuevo(),
        RegistroAuditado::Usuario(id),
        AccionAuditada::Edicion,
        cambios,
        sesion,
        reloj.ahora(),
    ));
}

/// El operador cambia su propia clave: debe saber la actual.
#[derive(Debug)]
pub struct CambiarClave<F, R, G, C> {
    fabrica: F,
    reloj: R,
    ids: G,
    claves: C,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds, C: Claves> CambiarClave<F, R, G, C> {
    pub const fn new(fabrica: F, reloj: R, ids: G, claves: C) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
            claves,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        actual: &str,
        nueva: &str,
    ) -> Result<(), ErrorUsuarios> {
        let id = sesion.operador();
        let mut uow = self.fabrica.nueva();
        let mut usuario = uow
            .usuarios()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let hash = cifrar(&self.claves, nueva, usuario.cedula())?;
        let actual_correcta = self.claves.verificar(actual, Some(usuario.clave()));
        let cambios = usuario
            .cambiar_clave(actual_correcta, hash)
            .map_err(ErrorCaso::Negocio)?;
        uow.usuarios().guardar(&usuario);
        anotar_edicion(&mut uow, &self.ids, &self.reloj, sesion, id, cambios);
        uow.confirmar().await.map_err(ErrorCaso::from)
    }
}

/// Otro operador le pone una clave temporal a un usuario (por
/// ejemplo, si la olvidó).
#[derive(Debug)]
pub struct RestablecerClave<F, R, G, C> {
    fabrica: F,
    reloj: R,
    ids: G,
    claves: C,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds, C: Claves> RestablecerClave<F, R, G, C> {
    pub const fn new(fabrica: F, reloj: R, ids: G, claves: C) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
            claves,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        id: OperadorId,
        temporal: &str,
    ) -> Result<(), ErrorUsuarios> {
        let mut uow = self.fabrica.nueva();
        let mut usuario = uow
            .usuarios()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let hash = cifrar(&self.claves, temporal, usuario.cedula())?;
        let cambios = usuario.restablecer_clave(hash);
        uow.usuarios().guardar(&usuario);
        anotar_edicion(&mut uow, &self.ids, &self.reloj, sesion, id, cambios);
        uow.confirmar().await.map_err(ErrorCaso::from)
    }
}

/// La lista de usuarios, por nombre.
#[derive(Debug)]
pub struct ListarUsuarios<F> {
    fabrica: F,
}

impl<F: FabricaUnidadDeTrabajo> ListarUsuarios<F> {
    pub const fn new(fabrica: F) -> Self {
        Self { fabrica }
    }

    pub async fn ejecutar(&self) -> Result<Vec<FilaUsuario>, ErrorCaso<Infallible>> {
        let mut uow = self.fabrica.nueva();
        let usuarios = uow.usuarios().todos().await?;
        Ok(usuarios
            .iter()
            .map(|usuario| FilaUsuario {
                id: usuario.id(),
                cedula: usuario.cedula().to_string(),
                nombre: usuario.nombre().to_string(),
                activo: usuario.activo(),
                debe_cambiar_clave: usuario.debe_cambiar_clave(),
            })
            .collect())
    }
}
