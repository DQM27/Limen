//! Contrato de los hechos: se guardan tal cual, se leen en el orden en que
//! ocurrieron y nunca se reemplazan.

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{
    Consultas, ErrorPersistencia, FabricaUnidadDeTrabajo, RegistroHechos, UnidadDeTrabajo,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::empresa_proveedora::EmpresaProveedoraId;
use limen_dominio::gafete::NumeroGafete;
use limen_dominio::hecho::{Hecho, HechoId};
use limen_dominio::ingreso_contratista::{IngresoContratista, IngresoGuardado, IngresoId};
use limen_dominio::ingreso_correo::{
    IngresoCorreo, IngresoCorreoGuardado, IngresoCorreoId, Motivo,
};
use limen_dominio::ingreso_proveedor::{
    IngresoProveedor, IngresoProveedorGuardado, IngresoProveedorId,
};
use limen_dominio::medio::{Medio, Placa};
use limen_dominio::movimiento::Marca;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::personal_kof::{CodigoEmpleado, PersonalKofId};
use limen_dominio::prestamo_kof::{PrestamoKof, PrestamoKofGuardado, PrestamoKofId};
use limen_dominio::visitante::Visitante;
use uuid::Uuid;

use crate::id_contratista;

// --- Datos de ejemplo ---

fn marca(texto: &str, operador: u128) -> Marca {
    let en: DateTime<Utc> = texto.parse().unwrap();
    Marca {
        en,
        operador: OperadorId::desde_uuid(Uuid::from_u128(operador)),
    }
}

fn id_hecho(n: u128) -> HechoId {
    HechoId::desde_uuid(Uuid::from_u128(9000 + n))
}

fn cedula(texto: &str) -> Cedula {
    Cedula::normalizar(texto).unwrap()
}

fn visitante(cedula_texto: &str, nombre: &str) -> Visitante {
    Visitante::restaurar(cedula(cedula_texto), NombrePersona::nuevo(nombre).unwrap())
}

const ENTRADA: &str = "2026-10-09T08:00:00Z";
const SALIDA: &str = "2026-10-09T17:00:00Z";

/// Un contratista en carro con gafete: todos los campos opcionales llenos.
fn ingreso_contratista(salida: Option<Marca>) -> IngresoContratista {
    IngresoContratista::restaurar(IngresoGuardado {
        id: IngresoId::desde_uuid(Uuid::from_u128(5001)),
        contratista: id_contratista(1),
        cedula: cedula("111111111"),
        medio: Medio::Vehiculo(Placa::nueva("ABC-123").unwrap()),
        gafete: Some(NumeroGafete::nuevo(7).unwrap()),
        entrada: marca(ENTRADA, 1),
        salida,
    })
}

fn ingreso_proveedor(salida: Option<Marca>) -> IngresoProveedor {
    IngresoProveedor::restaurar(IngresoProveedorGuardado {
        id: IngresoProveedorId::desde_uuid(Uuid::from_u128(6001)),
        visitante: visitante("222222222", "BETO SOLÍS"),
        empresa: EmpresaProveedoraId::desde_uuid(Uuid::from_u128(3001)),
        medio: Medio::APie,
        gafete: NumeroGafete::nuevo(4).unwrap(),
        entrada: marca(ENTRADA, 1),
        salida,
    })
}

fn ingreso_correo(salida: Option<Marca>) -> IngresoCorreo {
    IngresoCorreo::restaurar(IngresoCorreoGuardado {
        id: IngresoCorreoId::desde_uuid(Uuid::from_u128(7001)),
        visitante: visitante("333333333", "LUIS MORA"),
        motivo: Motivo::nuevo("Entrevista con RH").unwrap(),
        medio: Medio::APie,
        gafete: NumeroGafete::nuevo(2).unwrap(),
        entrada: marca(ENTRADA, 1),
        salida,
    })
}

fn prestamo_kof(devolucion: Option<Marca>) -> PrestamoKof {
    PrestamoKof::restaurar(PrestamoKofGuardado {
        id: PrestamoKofId::desde_uuid(Uuid::from_u128(8001)),
        personal: PersonalKofId::desde_uuid(Uuid::from_u128(4001)),
        codigo: CodigoEmpleado::nuevo("5040017").unwrap(),
        nombre: NombrePersona::nuevo("MICHAEL ARAYA").unwrap(),
        gafete: NumeroGafete::nuevo(3).unwrap(),
        entrega: marca(ENTRADA, 1),
        devolucion,
    })
}

/// Entrada y salida de cada una de las cuatro vías, en ese orden.
fn hechos_de_las_cuatro_vias() -> Vec<(Uuid, Vec<Hecho>)> {
    let salida = Some(marca(SALIDA, 2));
    vec![
        (
            Uuid::from_u128(5001),
            vec![
                Hecho::entrada_contratista(id_hecho(1), &ingreso_contratista(None)),
                Hecho::salida_contratista(id_hecho(2), &ingreso_contratista(salida)).unwrap(),
            ],
        ),
        (
            Uuid::from_u128(6001),
            vec![
                Hecho::entrada_proveedor(id_hecho(3), &ingreso_proveedor(None)),
                Hecho::salida_proveedor(id_hecho(4), &ingreso_proveedor(salida)).unwrap(),
            ],
        ),
        (
            Uuid::from_u128(7001),
            vec![
                Hecho::entrada_correo(id_hecho(5), &ingreso_correo(None)),
                Hecho::salida_correo(id_hecho(6), &ingreso_correo(salida)).unwrap(),
            ],
        ),
        (
            Uuid::from_u128(8001),
            vec![
                Hecho::entrega_kof(id_hecho(7), &prestamo_kof(None)),
                Hecho::devolucion_kof(id_hecho(8), &prestamo_kof(salida)).unwrap(),
            ],
        ),
    ]
}

// --- Pruebas ---

pub async fn los_hechos_de_las_cuatro_vias_se_guardan_y_se_leen_tal_cual<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    let esperados = hechos_de_las_cuatro_vias();
    // Se anotan al revés: el orden de lectura lo da el ID, no el de llegada.
    let mut uow = fabrica.nueva();
    for (_, hechos) in esperados.iter().rev() {
        for hecho in hechos.iter().rev() {
            uow.hechos().anotar(hecho.clone());
        }
    }
    uow.confirmar().await.unwrap();

    for (registro, hechos) in esperados {
        assert_eq!(
            fabrica.hechos_de(registro).await.unwrap(),
            hechos,
            "los hechos de {registro} vuelven tal cual y en orden"
        );
    }
    assert_eq!(
        fabrica.hechos_de(Uuid::from_u128(1)).await.unwrap(),
        Vec::new(),
        "un registro sin hechos no trae nada"
    );
}

pub async fn un_hecho_sin_confirmar_no_queda<F: FabricaUnidadDeTrabajo + Consultas>(fabrica: F) {
    let mut uow = fabrica.nueva();
    uow.hechos().anotar(Hecho::entrada_contratista(
        id_hecho(1),
        &ingreso_contratista(None),
    ));
    drop(uow);
    assert_eq!(
        fabrica.hechos_de(Uuid::from_u128(5001)).await.unwrap(),
        Vec::new(),
        "soltar la Unit of Work no escribe nada"
    );
}

pub async fn un_hecho_nunca_se_reemplaza_y_el_intento_no_aplica_nada<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    let original = Hecho::entrada_contratista(id_hecho(1), &ingreso_contratista(None));
    let mut uow = fabrica.nueva();
    uow.hechos().anotar(original.clone());
    uow.confirmar().await.unwrap();

    // Otro hecho con el mismo ID, junto con uno nuevo en la misma Unit of
    // Work: la base rechaza la transacción entera.
    let mut uow = fabrica.nueva();
    uow.hechos()
        .anotar(Hecho::entrada_correo(id_hecho(2), &ingreso_correo(None)));
    uow.hechos().anotar(Hecho::entrada_proveedor(
        id_hecho(1),
        &ingreso_proveedor(None),
    ));
    let resultado = uow.confirmar().await;
    assert!(
        matches!(resultado, Err(ErrorPersistencia::Tecnica(_))),
        "un ID repetido es un error técnico: {resultado:?}"
    );

    assert_eq!(
        fabrica.hechos_de(Uuid::from_u128(5001)).await.unwrap(),
        vec![original],
        "el hecho original sigue intacto"
    );
    assert_eq!(
        fabrica.hechos_de(Uuid::from_u128(7001)).await.unwrap(),
        Vec::new(),
        "nada de la transacción rechazada quedó guardado"
    );
}
