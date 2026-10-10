//! La lógica de los comandos de la app de escritorio, sin Tauri.
//!
//! Es un adaptador de entrada (arquitectura.md, sección 3): recibe lo que
//! manda la interfaz, lo traduce, llama al caso de uso de la [`Aplicacion`]
//! y devuelve JSON o un [`ErrorJson`]. No tiene reglas de negocio: si
//! aparece un `if` que decide algo, va al dominio.
//!
//! El cascarón de Tauri sólo registra estos métodos como comandos; por eso
//! todo se prueba aquí, con la base en memoria, sin abrir ninguna ventana.

pub mod campos;
mod dto;
mod error;
mod operador;

use std::fmt;

use limen_aplicacion::puertos::{Consultas, FabricaUnidadDeTrabajo, GeneradorIds, Reloj};
use limen_aplicacion::sesion::Sesion;
use limen_composicion::Aplicacion;
use limen_dominio::contratista::ContratistaId;
use limen_dominio::empresa::EmpresaId;
use limen_dominio::empresa_proveedora::EmpresaProveedoraId;
use limen_dominio::ingreso_contratista::IngresoId;
use limen_dominio::ingreso_correo::IngresoCorreoId;
use limen_dominio::ingreso_proveedor::IngresoProveedorId;
use limen_dominio::personal_kof::PersonalKofId;
use limen_dominio::presencia::Via;
use limen_dominio::prestamo_kof::PrestamoKofId;

pub use dto::{
    AccesoDto, CambioDto, CambioGafeteEntrada, ContratistaDto, ContratistaEntrada, EmpresaDto,
    EmpresaProveedoraDto, EntradaContratistaEntrada, EntradaCorreoEntrada, EntradaProveedorEntrada,
    EntradaRegistradaDto, FilaContratistaDto, GafeteDto, PersonaAdentroDto, PersonalKofDto,
};
pub use error::{ErrorEntrada, ErrorJson, TipoErrorJson};
pub use operador::{ErrorOperador, NOMBRE_PROVISIONAL, Operador, OperadorDelEquipo};

use campos::con_campo;
use dto::{leer_tipo_gafete, leer_uuid, leer_via};

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

    /// Todos los contratistas con el nombre de su empresa, para la grilla.
    pub async fn listar_contratistas(&self) -> Result<Vec<FilaContratistaDto>, ErrorJson> {
        let filas = self.app.contratistas.listar.ejecutar().await?;
        Ok(filas.iter().map(FilaContratistaDto::from).collect())
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
        let id = self
            .app
            .empresas
            .registrar
            .ejecutar(sesion, nombre)
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    /// Registra un contratista y devuelve su identificador. Un error de una
    /// regla trae el campo del formulario al que pertenece.
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
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    /// Edita un contratista con el formulario completo y devuelve lo que
    /// cambió: vacío si nada cambió (y entonces no se escribe nada). Los
    /// errores traen su campo, como al registrar.
    pub async fn editar_contratista(
        &self,
        sesion: &Sesion,
        id: &str,
        entrada: &ContratistaEntrada,
    ) -> Result<Vec<CambioDto>, ErrorJson> {
        let id = ContratistaId::desde_uuid(leer_uuid(id)?);
        let comando = entrada.a_comando()?;
        let cambios = self
            .app
            .contratistas
            .editar
            .ejecutar(sesion, id, &comando)
            .await
            .map_err(con_campo)?;
        Ok(cambios.iter().map(CambioDto::from).collect())
    }

    /// Registra la entrada de un contratista.
    pub async fn registrar_entrada_contratista(
        &self,
        sesion: &Sesion,
        entrada: &EntradaContratistaEntrada,
    ) -> Result<EntradaRegistradaDto, ErrorJson> {
        let comando = entrada.a_comando()?;
        let registrada = self
            .app
            .ingresos
            .entrada
            .ejecutar(sesion, &comando)
            .await
            .map_err(con_campo)?;
        Ok(registrada.into())
    }

    // --- Empresas de contratistas ---

    /// Cambia el nombre de una empresa; devuelve lo que cambió (vacío si el
    /// nombre ya era ese).
    pub async fn renombrar_empresa(
        &self,
        sesion: &Sesion,
        id: &str,
        nombre: &str,
    ) -> Result<Vec<CambioDto>, ErrorJson> {
        let id = EmpresaId::desde_uuid(leer_uuid(id)?);
        let cambios = self
            .app
            .empresas
            .renombrar
            .ejecutar(sesion, id, nombre)
            .await
            .map_err(con_campo)?;
        Ok(cambios.iter().map(CambioDto::from).collect())
    }

    // --- Proveedores ---

    /// Busca empresas proveedoras por nombre.
    pub async fn buscar_empresas_proveedoras(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<EmpresaProveedoraDto>, ErrorJson> {
        let encontradas = self
            .app
            .proveedores
            .buscar_empresas
            .ejecutar(texto, limite)
            .await?;
        Ok(encontradas.iter().map(EmpresaProveedoraDto::from).collect())
    }

    /// Registra una empresa proveedora y devuelve su identificador.
    pub async fn registrar_empresa_proveedora(
        &self,
        sesion: &Sesion,
        nombre: &str,
    ) -> Result<String, ErrorJson> {
        let id = self
            .app
            .proveedores
            .registrar_empresa
            .ejecutar(sesion, nombre)
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    /// Cambia el nombre de una empresa proveedora; devuelve lo que cambió.
    pub async fn renombrar_empresa_proveedora(
        &self,
        sesion: &Sesion,
        id: &str,
        nombre: &str,
    ) -> Result<Vec<CambioDto>, ErrorJson> {
        let id = EmpresaProveedoraId::desde_uuid(leer_uuid(id)?);
        let cambios = self
            .app
            .proveedores
            .renombrar_empresa
            .ejecutar(sesion, id, nombre)
            .await
            .map_err(con_campo)?;
        Ok(cambios.iter().map(CambioDto::from).collect())
    }

    /// Registra la entrada de un proveedor y devuelve el ingreso.
    pub async fn registrar_entrada_proveedor(
        &self,
        sesion: &Sesion,
        entrada: &EntradaProveedorEntrada,
    ) -> Result<String, ErrorJson> {
        let comando = entrada.a_comando()?;
        let id = self
            .app
            .proveedores
            .entrada
            .ejecutar(sesion, &comando)
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    // --- Ingreso por correo ---

    /// Registra la entrada de una visita autorizada por correo y devuelve el
    /// ingreso.
    pub async fn registrar_entrada_correo(
        &self,
        sesion: &Sesion,
        entrada: &EntradaCorreoEntrada,
    ) -> Result<String, ErrorJson> {
        let comando = entrada.a_comando()?;
        let id = self
            .app
            .correo
            .entrada
            .ejecutar(sesion, &comando)
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    // --- Personal KOF ---

    /// Busca personal KOF (activo o no) por código de empleado o nombre.
    pub async fn buscar_personal_kof(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<PersonalKofDto>, ErrorJson> {
        let encontrados = self.app.kof.buscar.ejecutar(texto, limite).await?;
        Ok(encontrados.iter().map(PersonalKofDto::from).collect())
    }

    /// Registra a una persona del personal KOF y devuelve su identificador.
    pub async fn registrar_personal_kof(
        &self,
        sesion: &Sesion,
        codigo_empleado: &str,
        nombre: &str,
    ) -> Result<String, ErrorJson> {
        let id = self
            .app
            .kof
            .registrar
            .ejecutar(sesion, codigo_empleado, nombre)
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    /// Edita el nombre y si está activa (se desactiva, no se borra: K2);
    /// devuelve lo que cambió.
    pub async fn editar_personal_kof(
        &self,
        sesion: &Sesion,
        id: &str,
        nombre: &str,
        activo: bool,
    ) -> Result<Vec<CambioDto>, ErrorJson> {
        let id = PersonalKofId::desde_uuid(leer_uuid(id)?);
        let cambios = self
            .app
            .kof
            .editar
            .ejecutar(sesion, id, nombre, activo)
            .await
            .map_err(con_campo)?;
        Ok(cambios.iter().map(CambioDto::from).collect())
    }

    /// Entrega el gafete provisional (es la entrada del personal KOF) y
    /// devuelve el préstamo.
    pub async fn entregar_gafete_kof(
        &self,
        sesion: &Sesion,
        personal_id: &str,
        gafete: u32,
    ) -> Result<String, ErrorJson> {
        let persona = PersonalKofId::desde_uuid(
            leer_uuid(personal_id)
                .map_err(|e| ErrorJson::from(e).en_campo(Some(campos::PERSONA)))?,
        );
        let id = self
            .app
            .kof
            .entregar_gafete
            .ejecutar(sesion, persona, gafete)
            .await
            .map_err(con_campo)?;
        Ok(id.uuid().to_string())
    }

    // --- Gafetes ---

    /// Los gafetes de un tipo, por número, con su estado y si están
    /// prestados.
    pub async fn listar_gafetes(&self, tipo: &str) -> Result<Vec<GafeteDto>, ErrorJson> {
        let tipo = leer_tipo_gafete(tipo)?;
        let gafetes = self.app.gafetes.listar.ejecutar(tipo).await?;
        Ok(gafetes.iter().map(GafeteDto::from).collect())
    }

    /// Crea los gafetes de un tipo de `desde` a `hasta` (hasta 1 000 de una
    /// vez, F1). Todo o nada: devuelve cuántos se crearon.
    pub async fn registrar_gafetes(
        &self,
        sesion: &Sesion,
        tipo: &str,
        desde: u32,
        hasta: u32,
    ) -> Result<usize, ErrorJson> {
        let tipo = leer_tipo_gafete(tipo)
            .map_err(|e| ErrorJson::from(e).en_campo(Some(campos::TIPO_GAFETE)))?;
        let creados = self
            .app
            .gafetes
            .registrar
            .ejecutar(sesion, tipo, desde, hasta)
            .await
            .map_err(con_campo)?;
        Ok(creados.len())
    }

    /// Cambia el estado de un gafete (perdido, pagado, apareció, de baja) y
    /// devuelve lo que cambió. Lo que falla es del gafete entero: el error
    /// no trae campo, salvo el deudor o el cambio ilegibles.
    pub async fn cambiar_gafete(
        &self,
        sesion: &Sesion,
        tipo: &str,
        numero: u32,
        cambio: &CambioGafeteEntrada,
    ) -> Result<Vec<CambioDto>, ErrorJson> {
        let tipo = leer_tipo_gafete(tipo)?;
        let cambio = cambio.a_cambio()?;
        let cambios = self
            .app
            .gafetes
            .cambiar
            .ejecutar(sesion, tipo, numero, cambio)
            .await?;
        Ok(cambios.iter().map(CambioDto::from).collect())
    }
}
