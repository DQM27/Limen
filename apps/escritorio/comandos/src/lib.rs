//! La lógica de los comandos de la app de escritorio, sin Tauri.
//!
//! Es un adaptador de entrada (arquitectura.md, sección 3): recibe lo que
//! manda la interfaz, lo traduce, llama al caso de uso de la [`Aplicacion`]
//! y devuelve JSON o un [`ErrorJson`]. No tiene reglas de negocio: si
//! aparece un `if` que decide algo, va al dominio.
//!
//! El cascarón de Tauri sólo registra estos métodos como comandos; por eso
//! todo se prueba aquí, con la base en memoria, sin abrir ninguna ventana.

mod dto;
mod error;

use std::fmt;

use limen_aplicacion::puertos::{Consultas, FabricaUnidadDeTrabajo, GeneradorIds, Reloj};
use limen_aplicacion::sesion::Sesion;
use limen_composicion::Aplicacion;
use limen_dominio::contratista::ContratistaId;
use limen_dominio::ingreso_contratista::IngresoId;
use limen_dominio::ingreso_correo::IngresoCorreoId;
use limen_dominio::ingreso_proveedor::IngresoProveedorId;
use limen_dominio::presencia::Via;
use limen_dominio::prestamo_kof::PrestamoKofId;

pub use dto::{
    AccesoDto, ContratistaDto, ContratistaEntrada, EmpresaDto, EntradaContratistaEntrada,
    EntradaRegistradaDto, PersonaAdentroDto,
};
pub use error::{ErrorEntrada, ErrorJson, TipoErrorJson};

use dto::{leer_uuid, leer_via};

/// Los comandos de la interfaz, sobre una aplicación ya armada.
///
/// Se comparte sin candado (por ejemplo dentro de un `Arc` en el estado de
/// Tauri): cada caso de uso crea su propia Unit of Work en cada llamada.
pub struct Comandos<F, R, G> {
    app: Aplicacion<F, R, G>,
}

impl<F, R, G> fmt::Debug for Comandos<F, R, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Comandos").finish_non_exhaustive()
    }
}

impl<F, R, G> Comandos<F, R, G>
where
    F: FabricaUnidadDeTrabajo + Consultas + Clone,
    R: Reloj + Clone,
    G: GeneradorIds + Clone,
{
    pub const fn new(app: Aplicacion<F, R, G>) -> Self {
        Self { app }
    }

    /// Quién está adentro ahora, por las cuatro vías juntas.
    pub async fn dentro(&self) -> Result<Vec<PersonaAdentroDto>, ErrorJson> {
        let adentro = self.app.quienes_estan_adentro.ejecutar().await?;
        Ok(adentro.iter().map(PersonaAdentroDto::from).collect())
    }

    /// Registra la salida de quien está adentro, desde la lista "dentro".
    /// `via` es el código de la vía (`CONTRATISTA`, `PROVEEDOR`, `CORREO`,
    /// `KOF`) e `ingreso_id` el que trae esa fila.
    pub async fn registrar_salida(
        &self,
        sesion: &Sesion,
        via: &str,
        ingreso_id: &str,
    ) -> Result<(), ErrorJson> {
        let via = leer_via(via)?;
        let id = leer_uuid(ingreso_id)?;
        match via {
            Via::Contratista => {
                let id = IngresoId::desde_uuid(id);
                self.app.ingresos.salida.ejecutar(sesion, id).await?;
            }
            Via::Proveedor => {
                let id = IngresoProveedorId::desde_uuid(id);
                self.app.proveedores.salida.ejecutar(sesion, id).await?;
            }
            Via::Correo => {
                let id = IngresoCorreoId::desde_uuid(id);
                self.app.correo.salida.ejecutar(sesion, id).await?;
            }
            Via::Kof => {
                let id = PrestamoKofId::desde_uuid(id);
                self.app.kof.devolver_gafete.ejecutar(sesion, id).await?;
            }
        }
        Ok(())
    }

    /// Registra la salida por el número del gafete que devuelve la persona.
    pub async fn registrar_salida_por_gafete(
        &self,
        sesion: &Sesion,
        via: &str,
        numero: u32,
    ) -> Result<(), ErrorJson> {
        match leer_via(via)? {
            Via::Contratista => self.app.ingresos.salida.por_gafete(sesion, numero).await?,
            Via::Proveedor => {
                self.app
                    .proveedores
                    .salida
                    .por_gafete(sesion, numero)
                    .await?;
            }
            Via::Correo => self.app.correo.salida.por_gafete(sesion, numero).await?,
            Via::Kof => {
                self.app
                    .kof
                    .devolver_gafete
                    .por_gafete(sesion, numero)
                    .await?;
            }
        }
        Ok(())
    }

    /// Busca contratistas por cédula (el inicio) o por nombre.
    pub async fn buscar_contratistas(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<ContratistaDto>, ErrorJson> {
        let encontrados = self.app.contratistas.buscar.ejecutar(texto, limite).await?;
        Ok(encontrados.iter().map(ContratistaDto::from).collect())
    }

    /// Busca empresas de contratistas por nombre.
    pub async fn buscar_empresas(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<EmpresaDto>, ErrorJson> {
        let encontradas = self.app.empresas.buscar.ejecutar(texto, limite).await?;
        Ok(encontradas.iter().map(EmpresaDto::from).collect())
    }

    /// Registra una empresa de contratistas y devuelve su identificador.
    pub async fn registrar_empresa(
        &self,
        sesion: &Sesion,
        nombre: &str,
    ) -> Result<String, ErrorJson> {
        let id = self.app.empresas.registrar.ejecutar(sesion, nombre).await?;
        Ok(id.uuid().to_string())
    }

    /// Registra un contratista y devuelve su identificador.
    pub async fn registrar_contratista(
        &self,
        sesion: &Sesion,
        entrada: &ContratistaEntrada,
    ) -> Result<String, ErrorJson> {
        let comando = entrada.a_comando()?;
        let id: ContratistaId = self
            .app
            .contratistas
            .registrar
            .ejecutar(sesion, &comando)
            .await?;
        Ok(id.uuid().to_string())
    }

    /// Registra la entrada de un contratista.
    pub async fn registrar_entrada_contratista(
        &self,
        sesion: &Sesion,
        entrada: &EntradaContratistaEntrada,
    ) -> Result<EntradaRegistradaDto, ErrorJson> {
        let comando = entrada.a_comando()?;
        let registrada = self.app.ingresos.entrada.ejecutar(sesion, &comando).await?;
        Ok(registrada.into())
    }
}
