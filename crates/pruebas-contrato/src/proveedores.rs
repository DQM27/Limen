//! Contrato de los repositorios de proveedores: su catálogo de empresas y
//! sus ingresos.

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{
    ErrorPersistencia, FabricaUnidadDeTrabajo, RepositorioEmpresas, RepositorioEmpresasProveedoras,
    RepositorioIngresosProveedor, Restriccion, UnidadDeTrabajo,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::empresa::NombreEmpresa;
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::NumeroGafete;
use limen_dominio::ingreso_proveedor::{
    IngresoProveedor, IngresoProveedorGuardado, IngresoProveedorId,
};
use limen_dominio::medio::{Medio, Placa};
use limen_dominio::movimiento::Marca;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::visitante::Visitante;
use uuid::Uuid;

use super::{empresa, sembrar};

// --- Datos de ejemplo ---

fn id_empresa_proveedora(n: u128) -> EmpresaProveedoraId {
    EmpresaProveedoraId::desde_uuid(Uuid::from_u128(3000 + n))
}

fn empresa_proveedora(n: u128, nombre: &str) -> EmpresaProveedora {
    EmpresaProveedora::restaurar(
        id_empresa_proveedora(n),
        NombreEmpresa::nuevo(nombre).unwrap(),
    )
}

fn marca(texto: &str) -> Marca {
    let en: DateTime<Utc> = texto.parse().unwrap();
    Marca {
        en,
        operador: OperadorId::desde_uuid(Uuid::from_u128(900)),
    }
}

/// Ingreso de ANA PEÑA (111111111) por la empresa proveedora 1, con el
/// gafete `gafete`.
fn ingreso(n: u128, gafete: u32, medio: Medio, salida: Option<Marca>) -> IngresoProveedor {
    IngresoProveedor::restaurar(IngresoProveedorGuardado {
        id: IngresoProveedorId::desde_uuid(Uuid::from_u128(6000 + n)),
        visitante: Visitante::restaurar(
            Cedula::normalizar("111111111").unwrap(),
            NombrePersona::nuevo("ANA PEÑA").unwrap(),
        ),
        empresa: id_empresa_proveedora(1),
        medio,
        gafete: NumeroGafete::nuevo(gafete).unwrap(),
        entrada: marca("2026-10-09T14:00:00Z"),
        salida,
    })
}

async fn sembrar_empresa_proveedora<F: FabricaUnidadDeTrabajo>(fabrica: &F) {
    let mut uow = fabrica.nueva();
    uow.empresas_proveedoras()
        .guardar(&empresa_proveedora(1, "GAS ZETA"));
    uow.confirmar().await.unwrap();
}

// --- Empresas proveedoras ---

pub async fn guarda_y_lee_una_empresa_proveedora<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_empresa_proveedora(&fabrica).await;
    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura
            .empresas_proveedoras()
            .obtener(id_empresa_proveedora(1))
            .await
            .unwrap(),
        Some(empresa_proveedora(1, "GAS ZETA")),
        "vuelve tal cual"
    );
    assert!(
        lectura
            .empresas_proveedoras()
            .existe(id_empresa_proveedora(1))
            .await
            .unwrap(),
        "existe"
    );
    assert!(
        !lectura
            .empresas_proveedoras()
            .existe(id_empresa_proveedora(2))
            .await
            .unwrap(),
        "otra no existe"
    );
    assert_eq!(
        lectura
            .empresas_proveedoras()
            .obtener(id_empresa_proveedora(2))
            .await
            .unwrap(),
        None,
        "lo inexistente no se encuentra"
    );
}

pub async fn nombre_de_empresa_proveedora_repetido_choca_solo_en_su_catalogo<
    F: FabricaUnidadDeTrabajo,
>(
    fabrica: F,
) {
    sembrar_empresa_proveedora(&fabrica).await;
    let mut otro_catalogo = fabrica.nueva();
    otro_catalogo.empresas().guardar(&empresa(1, "GAS ZETA"));
    assert_eq!(
        otro_catalogo.confirmar().await,
        Ok(()),
        "una empresa de contratistas puede llamarse igual"
    );

    let mut repetida = fabrica.nueva();
    repetida
        .empresas_proveedoras()
        .guardar(&empresa_proveedora(2, "GAS ZETA"));
    assert_eq!(
        repetida.confirmar().await,
        Err(ErrorPersistencia::Conflicto(
            Restriccion::NombreEmpresaProveedora
        )),
        "la base rechaza el nombre repetido en su catálogo"
    );
}

pub async fn nombre_de_empresa_proveedora_en_uso_excluye_a_la_propia<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    sembrar_empresa_proveedora(&fabrica).await;
    sembrar(&fabrica, &[]).await;
    let mut uow = fabrica.nueva();
    let nombre = NombreEmpresa::nuevo("GAS ZETA").unwrap();
    assert!(
        !uow.empresas_proveedoras()
            .nombre_en_uso(&nombre, Some(id_empresa_proveedora(1)))
            .await
            .unwrap(),
        "su propio nombre no cuenta"
    );
    assert!(
        uow.empresas_proveedoras()
            .nombre_en_uso(&nombre, None)
            .await
            .unwrap(),
        "para una nueva sí está en uso"
    );
    let acme = NombreEmpresa::nuevo("ACME").unwrap();
    assert!(
        !uow.empresas_proveedoras()
            .nombre_en_uso(&acme, None)
            .await
            .unwrap(),
        "ACME existe sólo como empresa de contratistas"
    );
}

// --- Ingresos de proveedores ---

pub async fn guarda_y_lee_un_ingreso_de_proveedor<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_empresa_proveedora(&fabrica).await;
    let en_carro = ingreso(
        1,
        4,
        Medio::Vehiculo(Placa::nueva("ABC-123").unwrap()),
        None,
    );
    let a_pie = ingreso(2, 5, Medio::APie, Some(marca("2026-10-09T15:00:00Z")));
    let mut uow = fabrica.nueva();
    uow.ingresos_proveedor().guardar(&en_carro);
    uow.ingresos_proveedor().guardar(&a_pie);
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    for esperado in [en_carro, a_pie] {
        assert_eq!(
            lectura
                .ingresos_proveedor()
                .obtener(esperado.id())
                .await
                .unwrap(),
            Some(esperado.clone()),
            "vuelve tal cual: {esperado:?}"
        );
    }
    assert_eq!(
        lectura
            .ingresos_proveedor()
            .obtener(IngresoProveedorId::desde_uuid(Uuid::from_u128(1)))
            .await
            .unwrap(),
        None,
        "lo inexistente no se encuentra"
    );
}

pub async fn abierto_con_gafete_de_proveedor_ignora_los_cerrados<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    sembrar_empresa_proveedora(&fabrica).await;
    let mut uow = fabrica.nueva();
    uow.ingresos_proveedor().guardar(&ingreso(
        1,
        4,
        Medio::APie,
        Some(marca("2026-10-09T15:00:00Z")),
    ));
    uow.ingresos_proveedor()
        .guardar(&ingreso(2, 4, Medio::APie, None));
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura
            .ingresos_proveedor()
            .abierto_con_gafete(NumeroGafete::nuevo(4).unwrap())
            .await
            .unwrap(),
        Some(ingreso(2, 4, Medio::APie, None)),
        "encuentra el que sigue abierto"
    );
    assert_eq!(
        lectura
            .ingresos_proveedor()
            .abierto_con_gafete(NumeroGafete::nuevo(5).unwrap())
            .await
            .unwrap(),
        None,
        "nadie tiene el 5"
    );
}
