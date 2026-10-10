//! Contrato de los repositorios de usuarios y de intentos fallidos de
//! inicio de sesión.

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{
    ErrorPersistencia, FabricaUnidadDeTrabajo, RepositorioIntentosInicio, RepositorioUsuarios,
    Restriccion, UnidadDeTrabajo,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::usuario::{HashClave, IntentosFallidos, Usuario, UsuarioGuardado};
use uuid::Uuid;

// --- Datos de ejemplo ---

fn id_usuario(n: u128) -> OperadorId {
    OperadorId::desde_uuid(Uuid::from_u128(6000 + n))
}

fn cedula(texto: &str) -> Cedula {
    Cedula::normalizar(texto).unwrap()
}

fn usuario(n: u128, cedula_texto: &str, nombre: &str) -> Usuario {
    Usuario::restaurar(UsuarioGuardado {
        id: id_usuario(n),
        cedula: cedula(cedula_texto),
        nombre: NombrePersona::nuevo(nombre).unwrap(),
        activo: true,
        clave: HashClave::desde_texto(format!("$argon2id$v=19$hash-{n}")),
        debe_cambiar_clave: false,
    })
}

async fn sembrar_usuarios<F: FabricaUnidadDeTrabajo>(fabrica: &F, usuarios: &[Usuario]) {
    let mut uow = fabrica.nueva();
    for usuario in usuarios {
        uow.usuarios().guardar(usuario);
    }
    uow.confirmar().await.unwrap();
}

// --- Usuarios ---

pub async fn guarda_y_lee_un_usuario_tal_cual<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let mut uow = fabrica.nueva();
    assert!(
        !uow.usuarios().hay_usuarios().await.unwrap(),
        "almacén vacío"
    );
    drop(uow);

    let mut ana = usuario(1, "111111111", "ANA MORA");
    sembrar_usuarios(&fabrica, &[ana.clone()]).await;
    let mut uow = fabrica.nueva();
    assert!(uow.usuarios().hay_usuarios().await.unwrap(), "ya hay uno");
    assert_eq!(
        uow.usuarios().obtener(id_usuario(1)).await.unwrap(),
        Some(ana.clone()),
        "se lee por ID tal cual se guardó"
    );
    assert_eq!(
        uow.usuarios()
            .obtener_por_cedula(&cedula("111111111"))
            .await
            .unwrap(),
        Some(ana.clone()),
        "se lee por cédula"
    );
    assert_eq!(
        uow.usuarios()
            .obtener_por_cedula(&cedula("222222222"))
            .await
            .unwrap(),
        None,
        "otra cédula no es de nadie"
    );
    assert!(
        uow.usuarios()
            .cedula_en_uso(&cedula("111111111"))
            .await
            .unwrap(),
        "la cédula está en uso"
    );
    drop(uow);

    // Guardar de nuevo actualiza (desactivar no borra: L2).
    ana.editar("ANA MORA", false, id_usuario(9)).unwrap();
    sembrar_usuarios(&fabrica, &[ana.clone()]).await;
    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.usuarios().obtener(id_usuario(1)).await.unwrap(),
        Some(ana),
        "guardar de nuevo actualiza"
    );
}

pub async fn la_cedula_de_usuario_repetida_choca<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_usuarios(&fabrica, &[usuario(1, "111111111", "ANA MORA")]).await;
    let mut uow = fabrica.nueva();
    uow.usuarios().guardar(&usuario(2, "111111111", "OTRA ANA"));
    assert_eq!(
        uow.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::CedulaUsuario)),
        "la base rechaza la cédula repetida"
    );
    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.usuarios().obtener(id_usuario(2)).await.unwrap(),
        None,
        "no se aplicó"
    );
}

pub async fn todos_los_usuarios_van_por_nombre<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_usuarios(
        &fabrica,
        &[
            usuario(1, "111111111", "BETO SOLIS"),
            usuario(2, "222222222", "ANA MORA"),
            usuario(3, "333333333", "CARLA RUIZ"),
        ],
    )
    .await;
    let mut uow = fabrica.nueva();
    let nombres: Vec<String> = uow
        .usuarios()
        .todos()
        .await
        .unwrap()
        .iter()
        .map(|usuario| usuario.nombre().to_string())
        .collect();
    assert_eq!(
        nombres,
        ["ANA MORA", "BETO SOLIS", "CARLA RUIZ"],
        "ordenados por nombre"
    );
}

// --- Intentos fallidos ---

fn instante(texto: &str) -> DateTime<Utc> {
    texto.parse().unwrap()
}

pub async fn los_intentos_fallidos_se_anotan_y_se_borran<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let ana = cedula("111111111");
    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.intentos_inicio().obtener(&ana).await.unwrap(),
        None,
        "sin intentos al empezar"
    );
    let intentos = IntentosFallidos {
        cantidad: 3,
        ultimo: instante("2026-10-10T08:00:00Z"),
    };
    uow.intentos_inicio().anotar(&ana, intentos);
    uow.confirmar().await.unwrap();

    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.intentos_inicio().obtener(&ana).await.unwrap(),
        Some(intentos),
        "se leen tal cual"
    );
    assert_eq!(
        uow.intentos_inicio()
            .obtener(&cedula("222222222"))
            .await
            .unwrap(),
        None,
        "son por cédula"
    );
    let mas = IntentosFallidos {
        cantidad: 4,
        ultimo: instante("2026-10-10T08:01:00Z"),
    };
    uow.intentos_inicio().anotar(&ana, mas);
    uow.confirmar().await.unwrap();

    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.intentos_inicio().obtener(&ana).await.unwrap(),
        Some(mas),
        "anotar reemplaza la cuenta"
    );
    uow.intentos_inicio().borrar(&ana);
    uow.confirmar().await.unwrap();

    let mut uow = fabrica.nueva();
    assert_eq!(
        uow.intentos_inicio().obtener(&ana).await.unwrap(),
        None,
        "borrados"
    );
    // Borrar lo que no existe no falla.
    uow.intentos_inicio().borrar(&ana);
    uow.confirmar().await.unwrap();
}
