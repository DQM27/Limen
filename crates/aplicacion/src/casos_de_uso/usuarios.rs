//! Casos de uso de los usuarios en el equipo (bloque L): iniciar sesión y
//! cambiar la propia clave. Nada más.
//!
//! Los usuarios son globales y vienen de la nube (L2): el equipo no los crea
//! ni los administra. En desarrollo, mientras no exista la nube, la base se
//! siembra con usuarios de prueba (`limen-composicion`, sólo en depuración).

use limen_dominio::auditoria::CambioCampo;
use limen_dominio::cedula::Cedula;
use limen_dominio::usuario::{
    ClaveNueva, ErrorInicioSesion, ErrorUsuario, HashClave, Usuario, cedula_de_usuario,
    decidir_inicio, sumar_fallo, verificar_bloqueo,
};

use crate::errores::ErrorCaso;
use crate::puertos::{
    AccionAuditada, Claves, EntradaAuditoria, FabricaUnidadDeTrabajo, GeneradorIds,
    RegistroAuditado, RegistroAuditoria, Reloj, RepositorioIntentosInicio, RepositorioUsuarios,
    UnidadDeTrabajo,
};
use crate::sesion::{OperadorId, Sesion};

pub type ErrorUsuarios = ErrorCaso<ErrorUsuario>;
pub type ErrorInicio = ErrorCaso<ErrorInicioSesion>;

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

// --- Cambio de clave ---

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
