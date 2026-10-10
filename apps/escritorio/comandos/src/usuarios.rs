//! Comandos de la sesión (bloque L): entrar, salir y cambiar la propia
//! clave. Los usuarios vienen de la nube: el equipo no los crea ni los
//! administra.

use limen_aplicacion::puertos::{Claves, Consultas, FabricaUnidadDeTrabajo, GeneradorIds, Reloj};

use crate::Comandos;
use crate::campos::con_campo;
use crate::error::ErrorJson;
use crate::sesion::UsuarioActual;

impl<F, R, G, C> Comandos<F, R, G, C>
where
    F: FabricaUnidadDeTrabajo + Consultas + Clone,
    R: Reloj + Clone,
    G: GeneradorIds + Clone,
    C: Claves + Clone,
{
    // --- Sesión ---

    /// Entra con cédula y clave. El error nunca dice cuál de las dos
    /// estaba mal, ni trae campo.
    pub async fn iniciar_sesion(
        &self,
        cedula: &str,
        clave: &str,
    ) -> Result<UsuarioActual, ErrorJson> {
        let iniciada = self
            .app
            .usuarios
            .iniciar_sesion
            .ejecutar(cedula, clave)
            .await?;
        Ok(self.sesion.abrir(&iniciada))
    }

    pub fn cerrar_sesion(&self) {
        self.sesion.cerrar();
    }

    /// Quién tiene la sesión, si alguien.
    pub fn usuario_actual(&self) -> Option<UsuarioActual> {
        self.sesion.actual()
    }

    /// Cambia la clave de quien tiene la sesión. Es lo único que se
    /// puede hacer con una clave temporal.
    pub async fn cambiar_clave(&self, clave_actual: &str, clave: &str) -> Result<(), ErrorJson> {
        let sesion = self.sesion.sesion_para_cambiar_clave()?;
        self.app
            .usuarios
            .cambiar_clave
            .ejecutar(&sesion, clave_actual, clave)
            .await
            .map_err(con_campo)?;
        self.sesion.clave_cambiada();
        Ok(())
    }
}
