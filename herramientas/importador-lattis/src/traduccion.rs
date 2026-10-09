//! De una fila de Lattis a una entidad de Limen.
//!
//! Todo pasa por los tipos del dominio (cédula, nombre, medio, gafete…), así
//! que lo que no cumple una regla se **rechaza con su código**, no se cuela.
//! Se carga con los constructores `restaurar`, que reconstruyen lo guardado
//! sin volver a exigir lo que pudo vencer con el tiempo (un PRAIND vencido, un
//! contratista sin acceso): para pruebas hacen falta, y en la realidad
//! existen. Un rechazo sólo lleva el identificador de Lattis y un código,
//! nunca datos de la persona.
//!
//! Reglas de adaptación acordadas con el dueño:
//!
//! - **Las empresas no desaparecen** (regla C): no se lee `activa`.
//! - **No existe el personal de ruta**: no se lee `es_personal_ruta`; esas
//!   personas son contratistas normales.
//! - `activo` del contratista pasa a ser `tiene_acceso`.
//! - Un tipo de ingreso que Limen no tiene (SWAT, POR CORREO: personal de otras empresas)
//!   entra como PRAIND con una **fecha ficticia lejana** si no trae fecha.
//! - Un ingreso en vehículo sin placa recibe una **placa inventada** (E2).
//! - Ingreso sin gafete es normal (E3): `NULL` se conserva.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use chrono::{DateTime, NaiveDate, Utc};
use limen_aplicacion::sesion::OperadorId;
use limen_dominio::contratista::{
    Contratista, ContratistaGuardado, ContratistaId, cedula_de_contratista,
};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{EstadoGafete, Gafete, NumeroGafete, TipoGafete};
use limen_dominio::ingreso_contratista::{IngresoContratista, IngresoGuardado, IngresoId};
use limen_dominio::ingreso_correo::{
    IngresoCorreo, IngresoCorreoGuardado, IngresoCorreoId, Motivo,
};
use limen_dominio::ingreso_proveedor::{
    IngresoProveedor, IngresoProveedorGuardado, IngresoProveedorId,
};
use limen_dominio::medio::{Medio, TipoMedio};
use limen_dominio::movimiento::Marca;
use limen_dominio::nombre::{NombreInvalido, NombrePersona};
use limen_dominio::personal_kof::{CodigoEmpleado, PersonalKof, PersonalKofId};
use limen_dominio::prestamo_kof::{PrestamoKofGuardado, PrestamoKofId};
use limen_dominio::tipo_ingreso::TipoIngreso;
use limen_dominio::visitante::Visitante;
use uuid::Uuid;

use crate::lectura::Fila;

/// Quien figura como autor de todo lo importado (la auditoría pide uno).
pub const OPERADOR_IMPORTADOR: Uuid = Uuid::from_u128(0x4c41_5454_4953_0000_0000_0000_0000_0001);

/// Placa que se inventa cuando un ingreso en vehículo no la trae.
pub const PLACA_INVENTADA: &str = "IMP-001";

pub const AJUSTE_PRAIND_FICTICIO: &str = "praind_ficticio";
pub const AJUSTE_PLACA_INVENTADA: &str = "placa_inventada";

/// Fecha de vencimiento que se da a quien no tiene PRAIND.
pub fn fecha_praind_ficticia() -> NaiveDate {
    NaiveDate::from_ymd_opt(2099, 12, 31).unwrap_or(NaiveDate::MAX)
}

/// Lo que salió mal con una fila: un código estable, sin datos personales.
pub type Resultado<T> = Result<T, String>;

/// Lo traducido y, si hubo que corregir algo para aceptarlo, cuál ajuste.
pub type Traducido<T> = Resultado<(T, Option<&'static str>)>;

// --- Lectura de valores ---

fn texto<'a>(fila: Fila<'a>, columna: &str) -> Resultado<&'a str> {
    fila.texto(columna).map_err(|error| error.to_string())
}

fn opcional<'a>(fila: Fila<'a>, columna: &str) -> Resultado<Option<&'a str>> {
    fila.opcional(columna).map_err(|error| error.to_string())
}

pub fn uuid_de(fila: Fila<'_>, columna: &str) -> Resultado<Uuid> {
    Uuid::parse_str(texto(fila, columna)?.trim()).map_err(|_| format!("{columna}_no_es_uuid"))
}

fn instante(texto: &str) -> Resultado<DateTime<Utc>> {
    DateTime::parse_from_str(texto.trim(), "%Y-%m-%d %H:%M:%S%.f%#z")
        .map(|instante| instante.with_timezone(&Utc))
        .map_err(|_| "instante_invalido".to_owned())
}

fn fecha(texto: &str) -> Resultado<NaiveDate> {
    NaiveDate::parse_from_str(texto.trim(), "%Y-%m-%d").map_err(|_| "fecha_invalida".to_owned())
}

fn booleano(texto: &str) -> Resultado<bool> {
    match texto.trim() {
        "true" | "t" => Ok(true),
        "false" | "f" => Ok(false),
        _ => Err("booleano_invalido".to_owned()),
    }
}

fn nombre_persona(texto: &str) -> Resultado<NombrePersona> {
    NombrePersona::nuevo(texto).map_err(|error| {
        match error {
            NombreInvalido::Vacio => "nombre_vacio",
            NombreInvalido::CaracteresNoPermitidos => "nombre_invalido",
        }
        .to_owned()
    })
}

fn nombre_empresa(texto: &str) -> Resultado<NombreEmpresa> {
    NombreEmpresa::nuevo(texto).map_err(|error| error.codigo().to_owned())
}

fn numero_gafete(texto: &str) -> Resultado<NumeroGafete> {
    let numero: u32 = texto
        .trim()
        .parse()
        .map_err(|_| "gafete_numero_invalido".to_owned())?;
    NumeroGafete::nuevo(numero).map_err(|error| error.codigo().to_owned())
}

fn gafete_opcional(fila: Fila<'_>) -> Resultado<Option<NumeroGafete>> {
    opcional(fila, "gafete_numero")?
        .map(numero_gafete)
        .transpose()
}

fn marca(en: DateTime<Utc>) -> Marca {
    Marca {
        en,
        operador: OperadorId::desde_uuid(OPERADOR_IMPORTADOR),
    }
}

/// Entrada y salida, y la salida no puede ser anterior a la entrada (E4).
fn marcas(fila: Fila<'_>) -> Resultado<(Marca, Option<Marca>)> {
    let entrada = instante(texto(fila, "hora_entrada")?)?;
    let salida = opcional(fila, "hora_salida")?.map(instante).transpose()?;
    if salida.is_some_and(|salida| salida < entrada) {
        return Err("salida_anterior_a_la_entrada".to_owned());
    }
    Ok((marca(entrada), salida.map(marca)))
}

/// El medio de un ingreso. Un vehículo sin placa, o con una que Limen no
/// acepta (E2), recibe una inventada: para pruebas importa el ingreso, no la
/// placa.
fn medio_de(tipo: TipoMedio, placa: Option<&str>) -> Resultado<(Medio, Option<&'static str>)> {
    if tipo == TipoMedio::Vehiculo {
        let propia = placa
            .map(str::trim)
            .filter(|placa| !placa.is_empty())
            .and_then(|placa| Medio::desde_formulario(tipo, Some(placa)).ok());
        if let Some(medio) = propia {
            return Ok((medio, None));
        }
        return Medio::desde_formulario(tipo, Some(PLACA_INVENTADA))
            .map(|medio| (medio, Some(AJUSTE_PLACA_INVENTADA)))
            .map_err(|error| error.codigo().to_owned());
    }
    Medio::desde_formulario(tipo, None)
        .map(|medio| (medio, None))
        .map_err(|error| error.codigo().to_owned())
}

/// Lattis anota el medio de los contratistas con una palabra (`CAMINANDO`,
/// `VEHICULO`); los proveedores y las visitas sólo traen la placa.
fn tipo_de_medio(texto: &str) -> Resultado<TipoMedio> {
    match texto.trim() {
        "CAMINANDO" | "A_PIE" => Ok(TipoMedio::APie),
        "VEHICULO" => Ok(TipoMedio::Vehiculo),
        _ => Err("medio_invalido".to_owned()),
    }
}

fn tipo_segun_placa(placa: Option<&str>) -> TipoMedio {
    if placa.is_some_and(|placa| !placa.trim().is_empty()) {
        TipoMedio::Vehiculo
    } else {
        TipoMedio::APie
    }
}

fn visitante(fila: Fila<'_>) -> Resultado<Visitante> {
    Visitante::nuevo(texto(fila, "cedula")?, texto(fila, "nombre")?)
        .map_err(|error| error.codigo().to_owned())
}

// --- Catálogos ---

pub fn empresa(fila: Fila<'_>) -> Resultado<Empresa> {
    Ok(Empresa::restaurar(
        EmpresaId::desde_uuid(uuid_de(fila, "id")?),
        nombre_empresa(texto(fila, "nombre")?)?,
    ))
}

pub fn empresa_proveedora(fila: Fila<'_>) -> Resultado<EmpresaProveedora> {
    Ok(EmpresaProveedora::restaurar(
        EmpresaProveedoraId::desde_uuid(uuid_de(fila, "id")?),
        nombre_empresa(texto(fila, "nombre")?)?,
    ))
}

pub fn gafete(fila: Fila<'_>) -> Resultado<Gafete> {
    let tipo = TipoGafete::desde_codigo(texto(fila, "tipo")?.trim())
        .ok_or_else(|| "gafete_tipo_invalido".to_owned())?;
    let estado = EstadoGafete::desde_codigo(texto(fila, "estado")?.trim())
        .ok_or_else(|| "gafete_estado_invalido".to_owned())?;
    Ok(Gafete::restaurar(
        tipo,
        numero_gafete(texto(fila, "numero")?)?,
        estado,
        None,
    ))
}

pub fn personal_kof(fila: Fila<'_>) -> Resultado<PersonalKof> {
    Ok(PersonalKof::restaurar(
        PersonalKofId::desde_uuid(uuid_de(fila, "id")?),
        CodigoEmpleado::nuevo(texto(fila, "codigo_empleado")?)
            .map_err(|error| error.codigo().to_owned())?,
        nombre_persona(texto(fila, "nombre")?)?,
        true,
    ))
}

/// El tipo y el vencimiento del PRAIND, con los ajustes para lo que Limen no
/// tiene: sin fecha no hay PRAIND (B5), salvo el personal de otras empresas
/// que entra con una fecha ficticia lejana.
fn tipo_y_praind(fila: Fila<'_>) -> Resultado<(TipoIngreso, NaiveDate, Option<&'static str>)> {
    let tipo = TipoIngreso::desde_codigo(texto(fila, "tipo_ingreso")?.trim());
    let vencimiento = opcional(fila, "fecha_vencimiento_praind")?;
    match (tipo, vencimiento) {
        (Ok(tipo), Some(vencimiento)) => Ok((tipo, fecha(vencimiento)?, None)),
        (Ok(_), None) => Err("praind_sin_fecha".to_owned()),
        (Err(_), Some(vencimiento)) => Ok((TipoIngreso::Praind, fecha(vencimiento)?, None)),
        (Err(_), None) => Ok((
            TipoIngreso::Praind,
            fecha_praind_ficticia(),
            Some(AJUSTE_PRAIND_FICTICIO),
        )),
    }
}

pub fn contratista(fila: Fila<'_>) -> Traducido<Contratista> {
    let (tipo_ingreso, fecha_vencimiento_praind, ajuste) = tipo_y_praind(fila)?;
    let guardado = ContratistaGuardado {
        id: ContratistaId::desde_uuid(uuid_de(fila, "id")?),
        cedula: cedula_de_contratista(texto(fila, "identificacion")?)
            .map_err(|error| error.codigo().to_owned())?,
        nombre: nombre_persona(texto(fila, "nombre")?)?,
        empresa: EmpresaId::desde_uuid(uuid_de(fila, "empresa_id")?),
        tipo_ingreso,
        fecha_vencimiento_praind,
        tiene_acceso: booleano(texto(fila, "activo")?)?,
    };
    Ok((Contratista::restaurar(guardado), ajuste))
}

// --- Movimientos ---

pub fn ingreso_contratista(
    fila: Fila<'_>,
    contratista: &Contratista,
) -> Traducido<IngresoContratista> {
    let (medio, ajuste) = medio_de(
        tipo_de_medio(texto(fila, "medio_ingreso")?)?,
        opcional(fila, "placa")?,
    )?;
    let (entrada, salida) = marcas(fila)?;
    // IN HOUSE no lleva gafete: el dominio se lo ignora al entrar (E3).
    let gafete = if contratista.requiere_gafete() {
        gafete_opcional(fila)?
    } else {
        None
    };
    let ingreso = IngresoContratista::restaurar(IngresoGuardado {
        id: IngresoId::desde_uuid(uuid_de(fila, "id")?),
        contratista: contratista.id(),
        cedula: contratista.cedula().clone(),
        medio,
        gafete,
        entrada,
        salida,
    });
    Ok((ingreso, ajuste))
}

/// Las empresas proveedoras ya cargadas, para enlazar los ingresos.
#[derive(Debug, Default)]
pub struct CatalogoProveedoras {
    ids: HashSet<Uuid>,
    por_nombre: HashMap<String, Uuid>,
}

impl CatalogoProveedoras {
    pub fn agregar(&mut self, empresa: &EmpresaProveedora) {
        let id = empresa.id().uuid();
        self.ids.insert(id);
        self.por_nombre
            .insert(empresa.nombre().as_str().to_owned(), id);
    }

    /// Por su identificador, o por su nombre cuando el ingreso no lo trae.
    fn buscar(&self, id: Option<&str>, nombre: &str) -> Option<EmpresaProveedoraId> {
        let por_id = id
            .and_then(|id| Uuid::parse_str(id.trim()).ok())
            .filter(|id| self.ids.contains(id));
        let uuid = por_id.or_else(|| {
            NombreEmpresa::nuevo(nombre)
                .ok()
                .and_then(|nombre| self.por_nombre.get(nombre.as_str()).copied())
        })?;
        Some(EmpresaProveedoraId::desde_uuid(uuid))
    }
}

pub fn ingreso_proveedor(
    fila: Fila<'_>,
    empresas: &CatalogoProveedoras,
) -> Traducido<IngresoProveedor> {
    let placa = opcional(fila, "placa")?;
    let (medio, ajuste) = medio_de(tipo_segun_placa(placa), placa)?;
    let (entrada, salida) = marcas(fila)?;
    let empresa = empresas
        .buscar(
            opcional(fila, "empresa_id")?,
            opcional(fila, "empresa_nombre")?.unwrap_or_default(),
        )
        .ok_or_else(|| "empresa_proveedora_no_importada".to_owned())?;
    let ingreso = IngresoProveedor::restaurar(IngresoProveedorGuardado {
        id: IngresoProveedorId::desde_uuid(uuid_de(fila, "id")?),
        visitante: visitante(fila)?,
        empresa,
        medio,
        gafete: numero_gafete(texto(fila, "gafete_numero")?)?,
        entrada,
        salida,
    });
    Ok((ingreso, ajuste))
}

pub fn ingreso_correo(fila: Fila<'_>) -> Traducido<IngresoCorreo> {
    let placa = opcional(fila, "placa")?;
    let (medio, ajuste) = medio_de(tipo_segun_placa(placa), placa)?;
    let (entrada, salida) = marcas(fila)?;
    let ingreso = IngresoCorreo::restaurar(IngresoCorreoGuardado {
        id: IngresoCorreoId::desde_uuid(uuid_de(fila, "id")?),
        visitante: visitante(fila)?,
        motivo: Motivo::nuevo(texto(fila, "motivo")?).map_err(|error| error.codigo().to_owned())?,
        medio,
        gafete: numero_gafete(texto(fila, "gafete_numero")?)?,
        entrada,
        salida,
    });
    Ok((ingreso, ajuste))
}

pub fn prestamo_kof<S: BuildHasher>(
    fila: Fila<'_>,
    personal: &HashMap<Uuid, PersonalKof, S>,
) -> Resultado<PrestamoKofGuardado> {
    let persona = personal
        .get(&uuid_de(fila, "personal_id")?)
        .ok_or_else(|| "personal_kof_no_importado".to_owned())?;
    let entrega = instante(texto(fila, "hora_entrega")?)?;
    let devolucion = opcional(fila, "hora_devolucion")?
        .map(instante)
        .transpose()?;
    if devolucion.is_some_and(|devolucion| devolucion < entrega) {
        return Err("devolucion_anterior_a_la_entrega".to_owned());
    }
    Ok(PrestamoKofGuardado {
        id: PrestamoKofId::desde_uuid(uuid_de(fila, "id")?),
        personal: persona.id(),
        codigo: persona.codigo().clone(),
        nombre: persona.nombre().clone(),
        gafete: numero_gafete(texto(fila, "gafete_numero")?)?,
        entrega: marca(entrega),
        devolucion: devolucion.map(marca),
    })
}
