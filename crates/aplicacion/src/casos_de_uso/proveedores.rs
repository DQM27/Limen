//! Casos de uso de proveedores: su catálogo de empresas y sus ingresos y
//! salidas.

use limen_dominio::auditoria::CambioCampo;
use limen_dominio::empresa::{ErrorEmpresa, HechosEmpresa, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, TipoGafete};
use limen_dominio::hecho::{Hecho, HechoId};
use limen_dominio::ingreso_proveedor::{
    DatosEntradaProveedor, ErrorIngresoProveedor, HechosEntradaProveedor, IngresoProveedor,
    IngresoProveedorId,
};
use limen_dominio::medio::TipoMedio;
use limen_dominio::movimiento::{ErrorSalida, Marca};
use limen_dominio::presencia::{Identidad, Via, YaEstaAdentro};
use limen_dominio::visitante::Visitante;

use super::gafetes::situacion_para_prestar;
use super::hora::sellar;
use super::veto::cedula_vetada;
use crate::errores::ErrorCaso;
use crate::puertos::{
    AccionAuditada, EntradaAuditoria, FabricaUnidadDeTrabajo, GeneradorIds, RegistroAuditado,
    RegistroAuditoria, RegistroHechos, Reloj, RepositorioEmpresasProveedoras, RepositorioGafetes,
    RepositorioIngresosProveedor, RepositorioPresencias, RepositorioReloj, Restriccion,
    UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorEmpresasProveedoras = ErrorCaso<ErrorEmpresa>;
pub type ErrorEntradaProveedor = ErrorCaso<ErrorIngresoProveedor>;
pub type ErrorSalidaProveedor = ErrorCaso<ErrorSalida>;

// --- Empresas proveedoras ---

/// La base rechazó el nombre porque otro equipo lo guardó primero.
fn conflicto_de_nombre(restriccion: Restriccion) -> Option<ErrorEmpresa> {
    (restriccion == Restriccion::NombreEmpresaProveedora).then_some(ErrorEmpresa::NombreRepetido)
}

#[derive(Debug)]
pub struct RegistrarEmpresaProveedora<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarEmpresaProveedora<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        nombre: &str,
    ) -> Result<EmpresaProveedoraId, ErrorEmpresasProveedoras> {
        let nombre_normalizado = NombreEmpresa::nuevo(nombre).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let hechos = HechosEmpresa {
            nombre_en_uso: uow
                .empresas_proveedoras()
                .nombre_en_uso(&nombre_normalizado, None)
                .await?,
        };
        let id = EmpresaProveedoraId::desde_uuid(self.ids.nuevo());
        let empresa =
            EmpresaProveedora::registrar(id, nombre, hechos).map_err(ErrorCaso::Negocio)?;

        uow.empresas_proveedoras().guardar(&empresa);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            RegistroAuditado::EmpresaProveedora(id),
            AccionAuditada::Alta,
            empresa.cambios_de_alta(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_nombre))?;
        Ok(id)
    }
}

#[derive(Debug)]
pub struct RenombrarEmpresaProveedora<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RenombrarEmpresaProveedora<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    /// Devuelve lo que cambió (vacío si el nombre ya era ese: entonces no se
    /// escribe nada).
    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        id: EmpresaProveedoraId,
        nombre: &str,
    ) -> Result<Vec<CambioCampo>, ErrorEmpresasProveedoras> {
        let nombre_normalizado = NombreEmpresa::nuevo(nombre).map_err(ErrorCaso::Negocio)?;
        let mut uow = self.fabrica.nueva();
        let mut empresa = uow
            .empresas_proveedoras()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let hechos = HechosEmpresa {
            nombre_en_uso: uow
                .empresas_proveedoras()
                .nombre_en_uso(&nombre_normalizado, Some(id))
                .await?,
        };
        let cambios = empresa
            .renombrar(nombre, hechos)
            .map_err(ErrorCaso::Negocio)?;
        if cambios.is_empty() {
            return Ok(cambios);
        }

        uow.empresas_proveedoras().guardar(&empresa);
        uow.auditoria().anotar(EntradaAuditoria::nueva(
            self.ids.nuevo(),
            RegistroAuditado::EmpresaProveedora(id),
            AccionAuditada::Edicion,
            cambios.clone(),
            sesion,
            self.reloj.ahora(),
        ));
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_nombre))?;
        Ok(cambios)
    }
}

// --- Ingresos de proveedores ---

/// La base rechazó la entrada porque otro equipo registró a la misma
/// persona o prestó el mismo gafete primero.
fn conflicto_de_entrada(restriccion: Restriccion) -> Option<ErrorIngresoProveedor> {
    match restriccion {
        Restriccion::PresenciaPersona => Some(ErrorIngresoProveedor::YaEstaAdentro(YaEstaAdentro(
            Via::Proveedor,
        ))),
        Restriccion::GafetePrestado => {
            Some(ErrorIngresoProveedor::Gafete(ErrorPrestamoGafete::Prestado))
        }
        _ => None,
    }
}

/// Lo que llega del formulario de entrada de un proveedor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComandoEntradaProveedor {
    pub cedula: String,
    pub nombre: String,
    pub empresa: EmpresaProveedoraId,
    pub medio: TipoMedio,
    pub placa: Option<String>,
    /// Número del gafete de proveedor (obligatorio).
    pub gafete: u32,
}

#[derive(Debug)]
pub struct RegistrarEntradaProveedor<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarEntradaProveedor<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        comando: &ComandoEntradaProveedor,
    ) -> Result<IngresoProveedorId, ErrorEntradaProveedor> {
        let visitante = Visitante::nuevo(&comando.cedula, &comando.nombre)
            .map_err(|error| ErrorCaso::Negocio(ErrorIngresoProveedor::Visitante(error)))?;
        // Un gafete 0 no existe en ningún catálogo.
        let gafete = NumeroGafete::nuevo(comando.gafete).map_err(|_| {
            ErrorCaso::Negocio(ErrorIngresoProveedor::Gafete(
                ErrorPrestamoGafete::NoRegistrado,
            ))
        })?;

        let mut uow = self.fabrica.nueva();
        let cedula = visitante.cedula();
        let hechos = HechosEntradaProveedor {
            empresa_existe: uow.empresas_proveedoras().existe(comando.empresa).await?,
            adentro_por: uow
                .presencias()
                .via_adentro(&Identidad::from(cedula))
                .await?,
            vetada: cedula_vetada(uow.contratistas(), cedula).await?,
            situacion_gafete: situacion_para_prestar(uow.gafetes(), TipoGafete::Proveedor, gafete)
                .await?,
        };
        let hora = sellar(&mut uow, &self.reloj).await?;
        let marca = Marca {
            en: hora.en,
            operador: sesion.operador(),
        };
        let datos = DatosEntradaProveedor {
            empresa: comando.empresa,
            medio: comando.medio,
            placa: comando.placa.as_deref(),
            gafete,
        };
        let ingreso = IngresoProveedor::registrar_entrada(
            IngresoProveedorId::desde_uuid(self.ids.nuevo()),
            visitante,
            datos,
            hechos,
            marca,
        )
        .map_err(ErrorCaso::Negocio)?;

        uow.ingresos_proveedor().guardar(&ingreso);
        uow.hechos().anotar(Hecho::entrada_proveedor(
            HechoId::desde_uuid(self.ids.nuevo()),
            &ingreso,
            hora.into(),
        ));
        uow.presencias().anotar_entrada(
            &Identidad::from(ingreso.cedula()),
            Via::Proveedor,
            marca.en,
        );
        uow.gafetes()
            .anotar_prestamo(TipoGafete::Proveedor, gafete, marca.en);
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_entrada))?;
        Ok(ingreso.id())
    }
}

/// Registra la salida de un proveedor: por el ingreso (desde la lista de
/// quienes están adentro) o por el número del gafete que devuelve.
#[derive(Debug)]
pub struct RegistrarSalidaProveedor<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarSalidaProveedor<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        id: IngresoProveedorId,
    ) -> Result<(), ErrorSalidaProveedor> {
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos_proveedor()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    /// Salida por el gafete de proveedor que devuelve la persona.
    pub async fn por_gafete(
        &self,
        sesion: &Sesion,
        numero: u32,
    ) -> Result<(), ErrorSalidaProveedor> {
        let numero = NumeroGafete::nuevo(numero).map_err(|_| ErrorCaso::NoEncontrado)?;
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos_proveedor()
            .abierto_con_gafete(numero)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    async fn cerrar<U: UnidadDeTrabajo>(
        &self,
        sesion: &Sesion,
        mut uow: U,
        mut ingreso: IngresoProveedor,
    ) -> Result<(), ErrorSalidaProveedor> {
        let hora = sellar(&mut uow, &self.reloj).await?;
        let marca = Marca {
            en: hora.en,
            operador: sesion.operador(),
        };
        ingreso
            .registrar_salida(marca)
            .map_err(ErrorCaso::Negocio)?;

        uow.ingresos_proveedor().guardar(&ingreso);
        if let Some(hecho) =
            Hecho::salida_proveedor(HechoId::desde_uuid(self.ids.nuevo()), &ingreso, hora.into())
        {
            uow.hechos().anotar(hecho);
        }
        uow.presencias()
            .anotar_salida(&Identidad::from(ingreso.cedula()));
        uow.gafetes()
            .anotar_devolucion(TipoGafete::Proveedor, ingreso.gafete());
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar().await.map_err(ErrorCaso::from)
    }
}
