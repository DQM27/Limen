//! Pruebas de contrato de la persistencia.
//!
//! Cada función de esta batería describe una promesa de los puertos de
//! persistencia (`FabricaUnidadDeTrabajo` y sus repositorios). La misma
//! batería corre contra todos los adaptadores, con la macro
//! [`bateria_de_contrato!`]:
//!
//! ```ignore
//! limen_pruebas_contrato::bateria_de_contrato!(AlmacenMemoria::new());
//! ```
//!
//! Si el doble en memoria y `SurrealDB` se comportaran distinto, alguna de
//! estas pruebas fallaría en uno de los dos. Así las pruebas de los casos
//! de uso, que corren contra el doble, valen también para la base real.
//!
//! Las pruebas sólo usan los puertos: siembran datos con una Unit of Work y
//! los leen con otra, como lo haría un caso de uso.

#![expect(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    reason = "batería de pruebas: entrar en pánico es la prueba diciendo que algo está mal"
)]

use chrono::NaiveDate;
use limen_aplicacion::puertos::{
    ConsultaPresencias, ErrorPersistencia, FabricaUnidadDeTrabajo, RepositorioContratistas,
    RepositorioEmpresas, Restriccion, UnidadDeTrabajo,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaGuardado, ContratistaId};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::nombre::NombrePersona;
use limen_dominio::tipo_ingreso::TipoIngreso;
use uuid::Uuid;

/// Genera un `#[tokio::test]` por cada prueba de la batería. `$fabrica` es
/// una expresión (puede usar `.await`) que crea un almacén nuevo y vacío.
#[macro_export]
macro_rules! bateria_de_contrato {
    ($fabrica:expr) => {
        $crate::bateria_de_contrato!(@pruebas $fabrica;
            sin_confirmar_no_se_escribe_nada,
            confirmar_sin_cambios_no_falla,
            guarda_y_lee_un_contratista_tal_cual,
            guarda_y_lee_una_empresa,
            lo_inexistente_no_se_encuentra,
            una_lectura_no_ve_lo_anotado_en_la_misma_uow,
            guardar_de_nuevo_actualiza,
            un_contratista_y_su_empresa_nueva_en_la_misma_uow,
            cedula_en_uso_excluye_al_propio_contratista,
            cedula_repetida_choca_y_no_aplica_nada,
            dos_pendientes_con_la_misma_cedula_chocan,
            intercambiar_cedulas_en_una_misma_uow_choca,
            nombre_de_empresa_repetido_choca,
            nombre_en_uso_excluye_a_la_propia_empresa,
        );
    };
    (@pruebas $fabrica:expr; $($prueba:ident),+ $(,)?) => {
        $(
            #[tokio::test]
            async fn $prueba() {
                $crate::$prueba($fabrica).await;
            }
        )+
    };
}

// --- Datos de ejemplo ---

fn id_contratista(n: u128) -> ContratistaId {
    ContratistaId::desde_uuid(Uuid::from_u128(n))
}

fn id_empresa(n: u128) -> EmpresaId {
    EmpresaId::desde_uuid(Uuid::from_u128(1000 + n))
}

fn contratista(n: u128, cedula: &str, nombre: &str) -> Contratista {
    Contratista::restaurar(ContratistaGuardado {
        id: id_contratista(n),
        cedula: Cedula::normalizar(cedula).unwrap(),
        nombre: NombrePersona::nuevo(nombre).unwrap(),
        empresa: id_empresa(1),
        tipo_ingreso: TipoIngreso::InHouse,
        fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2027, 3, 15).unwrap(),
        tiene_acceso: false,
    })
}

fn empresa(n: u128, nombre: &str) -> Empresa {
    Empresa::restaurar(id_empresa(n), NombreEmpresa::nuevo(nombre).unwrap())
}

/// Siembra la empresa 1 y los contratistas dados, en una sola Unit of Work.
async fn sembrar<F: FabricaUnidadDeTrabajo>(fabrica: &F, contratistas: &[Contratista]) {
    let mut uow = fabrica.nueva();
    uow.empresas().guardar(&empresa(1, "ACME"));
    for c in contratistas {
        uow.contratistas().guardar(c);
    }
    uow.confirmar().await.unwrap();
}

// --- La batería ---

pub async fn sin_confirmar_no_se_escribe_nada<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let mut uow = fabrica.nueva();
    uow.empresas().guardar(&empresa(1, "ACME"));
    uow.contratistas()
        .guardar(&contratista(1, "111111111", "ANA"));
    drop(uow);

    let mut lectura = fabrica.nueva();
    assert!(
        !lectura.empresas().existe(id_empresa(1)).await.unwrap(),
        "soltar la UoW sin confirmar equivale a cancelar"
    );
    assert_eq!(
        lectura
            .contratistas()
            .obtener(id_contratista(1))
            .await
            .unwrap(),
        None,
        "tampoco se guardó el contratista"
    );
}

pub async fn confirmar_sin_cambios_no_falla<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    assert_eq!(
        fabrica.nueva().confirmar().await,
        Ok(()),
        "una UoW vacía confirma"
    );
}

pub async fn guarda_y_lee_un_contratista_tal_cual<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let original = contratista(1, "111111111", "ANA PEÑA");
    sembrar(&fabrica, std::slice::from_ref(&original)).await;

    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.contratistas().obtener(id_contratista(1)).await.unwrap(),
        Some(original.clone()),
        "por ID vuelve idéntico: fecha, tipo, acceso y Ñ incluidos"
    );
    assert_eq!(
        uow.contratistas()
            .obtener_por_cedula(original.cedula())
            .await
            .unwrap(),
        Some(original),
        "por cédula también"
    );
}

pub async fn guarda_y_lee_una_empresa<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[]).await;
    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.empresas().obtener(id_empresa(1)).await.unwrap(),
        Some(empresa(1, "ACME")),
        "vuelve idéntica"
    );
    assert!(
        uow.empresas().existe(id_empresa(1)).await.unwrap(),
        "existe"
    );
    assert!(
        uow.empresas()
            .nombre_en_uso(&NombreEmpresa::nuevo("ACME").unwrap(), None)
            .await
            .unwrap(),
        "su nombre está en uso"
    );
}

pub async fn lo_inexistente_no_se_encuentra<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let mut uow = fabrica.nueva();
    let cedula = Cedula::normalizar("999999999").unwrap();
    assert_eq!(
        uow.contratistas().obtener(id_contratista(9)).await.unwrap(),
        None,
        "por ID"
    );
    assert_eq!(
        uow.contratistas()
            .obtener_por_cedula(&cedula)
            .await
            .unwrap(),
        None,
        "por cédula"
    );
    assert!(
        !uow.contratistas()
            .cedula_en_uso(&cedula, None)
            .await
            .unwrap(),
        "la cédula está libre"
    );
    assert_eq!(
        uow.empresas().obtener(id_empresa(9)).await.unwrap(),
        None,
        "empresa"
    );
    assert!(
        !uow.empresas().existe(id_empresa(9)).await.unwrap(),
        "no existe"
    );
    assert!(
        !uow.presencias()
            .esta_adentro(id_contratista(9))
            .await
            .unwrap(),
        "nadie está adentro"
    );
}

pub async fn una_lectura_no_ve_lo_anotado_en_la_misma_uow<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let mut uow = fabrica.nueva();
    uow.empresas().guardar(&empresa(1, "ACME"));
    assert!(
        !uow.empresas().existe(id_empresa(1)).await.unwrap(),
        "sólo se ve lo confirmado"
    );
}

pub async fn guardar_de_nuevo_actualiza<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let mut uow = fabrica.nueva();
    let editado = contratista(1, "111111111", "ANA SOLANO");
    uow.contratistas().guardar(&editado);
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura
            .contratistas()
            .obtener(id_contratista(1))
            .await
            .unwrap(),
        Some(editado),
        "el mismo ID se actualiza, no se duplica"
    );
}

pub async fn un_contratista_y_su_empresa_nueva_en_la_misma_uow<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    let mut uow = fabrica.nueva();
    uow.contratistas()
        .guardar(&contratista(1, "111111111", "ANA"));
    uow.empresas().guardar(&empresa(1, "ACME"));
    assert_eq!(
        uow.confirmar().await,
        Ok(()),
        "el orden de anotación no importa: la empresa existe al confirmar"
    );
}

pub async fn cedula_en_uso_excluye_al_propio_contratista<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let mut uow = fabrica.nueva();
    let cedula = Cedula::normalizar("111111111").unwrap();
    assert!(
        uow.contratistas()
            .cedula_en_uso(&cedula, None)
            .await
            .unwrap(),
        "la usa alguien"
    );
    assert!(
        !uow.contratistas()
            .cedula_en_uso(&cedula, Some(id_contratista(1)))
            .await
            .unwrap(),
        "su propia cédula no cuenta"
    );
    assert!(
        uow.contratistas()
            .cedula_en_uso(&cedula, Some(id_contratista(2)))
            .await
            .unwrap(),
        "para otro contratista sí está en uso"
    );
}

pub async fn cedula_repetida_choca_y_no_aplica_nada<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let mut uow = fabrica.nueva();
    uow.empresas().guardar(&empresa(2, "OTRA"));
    uow.contratistas()
        .guardar(&contratista(2, "111111111", "BETO"));
    assert_eq!(
        uow.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::CedulaContratista)),
        "la base rechaza la cédula repetida"
    );

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura
            .contratistas()
            .obtener(id_contratista(2))
            .await
            .unwrap(),
        None,
        "el contratista no se guardó"
    );
    assert!(
        !lectura.empresas().existe(id_empresa(2)).await.unwrap(),
        "ni lo demás de la transacción: todo o nada"
    );
}

pub async fn dos_pendientes_con_la_misma_cedula_chocan<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[]).await;
    let mut uow = fabrica.nueva();
    uow.contratistas()
        .guardar(&contratista(1, "111111111", "ANA"));
    uow.contratistas()
        .guardar(&contratista(2, "111111111", "BETO"));
    assert_eq!(
        uow.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::CedulaContratista)),
        "también dentro de la misma transacción"
    );
}

/// La unicidad se revisa sentencia por sentencia, como en `SurrealDB`: dos
/// contratistas no pueden intercambiar cédulas en una sola transacción
/// (el primero chocaría con el segundo antes de que éste cambie). Ningún
/// caso de uso lo necesita; la prueba deja fijado el comportamiento.
pub async fn intercambiar_cedulas_en_una_misma_uow_choca<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(
        &fabrica,
        &[
            contratista(1, "111111111", "ANA"),
            contratista(2, "222222222", "BETO"),
        ],
    )
    .await;
    let mut uow = fabrica.nueva();
    uow.contratistas()
        .guardar(&contratista(1, "222222222", "ANA"));
    uow.contratistas()
        .guardar(&contratista(2, "111111111", "BETO"));
    assert_eq!(
        uow.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::CedulaContratista)),
        "la unicidad se revisa en cada paso"
    );
}

pub async fn nombre_de_empresa_repetido_choca<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[]).await;
    let mut uow = fabrica.nueva();
    uow.empresas().guardar(&empresa(2, "ACME"));
    assert_eq!(
        uow.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::NombreEmpresa)),
        "la base rechaza el nombre repetido"
    );
}

pub async fn nombre_en_uso_excluye_a_la_propia_empresa<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[]).await;
    let mut uow = fabrica.nueva();
    let nombre = NombreEmpresa::nuevo("ACME").unwrap();
    assert!(
        !uow.empresas()
            .nombre_en_uso(&nombre, Some(id_empresa(1)))
            .await
            .unwrap(),
        "su propio nombre no cuenta"
    );
    assert!(
        uow.empresas()
            .nombre_en_uso(&nombre, Some(id_empresa(2)))
            .await
            .unwrap(),
        "para otra empresa sí está en uso"
    );
}
