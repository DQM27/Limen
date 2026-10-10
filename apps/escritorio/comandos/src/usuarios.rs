//! Comandos de usuarios e inicio de sesión (bloque L).
//!
//! TEMPORAL: mientras no exista el panel de la nube, los usuarios se
//! administran desde el equipo, con cualquier sesión abierta (L1: hay un
//! solo rol).

use limen_aplicacion::puertos::{Claves, Consultas, FabricaUnidadDeTrabajo, GeneradorIds, Reloj};
use limen_aplicacion::sesion::{OperadorId, Sesion};

use crate::Comandos;
use crate::campos::con_campo;
use crate::dto::{CambioDto, UsuarioDto, UsuarioEntrada, leer_uuid};
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

    /// Si el equipo ya tiene usuarios. Sin ninguno, la interfaz muestra el
    /// alta del primero en vez del inicio de sesión. No pide sesión.
    pub async fn hay_usuarios(&self) -> Result<bool, ErrorJson> {
        Ok(self.app.usuarios.hay_usuarios.ejecutar().await?)
    }

    /// Crea el primer usuario de un equipo recién instalado y le abre la
    /// sesión. Sólo funciona mientras no haya ningún usuario.
    pub async fn crear_primer_usuario(
        &self,
        entrada: &UsuarioEntrada,
    ) -> Result<UsuarioActual, ErrorJson> {
        let iniciada = self
            .app
            .usuarios
            .crear_primero
            .ejecutar(&entrada.cedula, &entrada.nombre, &entrada.clave)
            .await
            .map_err(con_campo)?;
        Ok(self.sesion.abrir(&iniciada))
    }

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

    // --- Administración (TEMPORAL, hasta el panel de la nube) ---

    /// Todos los usuarios, por nombre.
    pub async fn listar_usuarios(&self) -> Result<Vec<UsuarioDto>, ErrorJson> {
        let filas = self.app.usuarios.listar.ejecutar().await?;
        Ok(filas.iter().map(UsuarioDto::from).collect())
    }

    /// Registra un usuario con una clave temporal y devuelve su ID.
    pub async fn registrar_usuario(
        &self,
        sesion: &Sesion,
        entrada: &UsuarioEntrada,
    ) -> Result<String, ErrorJson> {
        let id = self
            .app
            .usuarios
            .registrar
            .ejecutar(sesion, &entrada.cedula, &entrada.nombre, &entrada.clave)
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    /// Edita el nombre y si está activo (se desactiva, no se borra: L2);
    /// devuelve lo que cambió.
    pub async fn editar_usuario(
        &self,
        sesion: &Sesion,
        id: &str,
        nombre: &str,
        activo: bool,
    ) -> Result<Vec<CambioDto>, ErrorJson> {
        let id = OperadorId::desde_uuid(leer_uuid(id)?);
        let cambios = self
            .app
            .usuarios
            .editar
            .ejecutar(sesion, id, nombre, activo)
            .await
            .map_err(con_campo)?;
        Ok(cambios.iter().map(CambioDto::from).collect())
    }

    /// Le pone una clave temporal a otro usuario (por ejemplo, si
    /// olvidó la suya). Deberá cambiarla al entrar.
    pub async fn restablecer_clave(
        &self,
        sesion: &Sesion,
        id: &str,
        clave: &str,
    ) -> Result<(), ErrorJson> {
        let id = OperadorId::desde_uuid(leer_uuid(id)?);
        self.app
            .usuarios
            .restablecer_clave
            .ejecutar(sesion, id, clave)
            .await
            .map_err(con_campo)
    }
}
