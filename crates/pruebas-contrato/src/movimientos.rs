//! Contrato de presencias, gafetes, ingresos y reloj: lo que hace cumplir
//! "una persona adentro una sola vez" y "un gafete prestado una sola vez"
//! aunque dos equipos registren al mismo tiempo.

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{
    ErrorPersistencia, FabricaUnidadDeTrabajo, RepositorioGafetes, RepositorioIngresos,
    RepositorioPersonalKof, RepositorioPresencias, RepositorioReloj, Restriccion, UnidadDeTrabajo,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::gafete::{EstadoGafete, Gafete, NumeroGafete, Portador, TipoGafete};
use limen_dominio::ingreso_contratista::{IngresoContratista, IngresoGuardado, IngresoId};
use limen_dominio::medio::{Medio, Placa};
use limen_dominio::movimiento::Marca;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::personal_kof::{CodigoEmpleado, PersonalKof, PersonalKofId};
use limen_dominio::presencia::{Identidad, Via};
use uuid::Uuid;

use super::{contratista, id_contratista, sembrar};

// --- Datos de ejemplo ---

fn instante(texto: &str) -> DateTime<Utc> {
    texto.parse().unwrap()
}

fn cedula(texto: &str) -> Cedula {
    Cedula::normalizar(texto).unwrap()
}

fn numero(n: u32) -> NumeroGafete {
    NumeroGafete::nuevo(n).unwrap()
}

fn disponible(tipo: TipoGafete, n: u32) -> Gafete {
    Gafete::restaurar(tipo, numero(n), EstadoGafete::Disponible, None)
}

fn marca(texto: &str) -> Marca {
    Marca {
        en: instante(texto),
        operador: OperadorId::desde_uuid(Uuid::from_u128(900)),
    }
}

/// Ingreso del contratista 1 (cédula 111111111), en carro y con gafete 7.
fn ingreso(n: u128, salida: Option<Marca>) -> IngresoContratista {
    IngresoContratista::restaurar(IngresoGuardado {
        id: IngresoId::desde_uuid(Uuid::from_u128(5000 + n)),
        contratista: id_contratista(1),
        cedula: cedula("111111111"),
        medio: Medio::Vehiculo(Placa::nueva("ABC-123").unwrap()),
        gafete: Some(numero(7)),
        entrada: marca("2026-10-09T14:00:00Z"),
        salida,
    })
}

// --- Presencias ---

pub async fn una_persona_entra_y_sale<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let persona = Identidad::from(&cedula("111111111"));
    let mut uow = fabrica.nueva();
    uow.presencias()
        .anotar_entrada(&persona, Via::Proveedor, instante("2026-10-09T14:00:00Z"));
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura.presencias().via_adentro(&persona).await.unwrap(),
        Some(Via::Proveedor),
        "quedó adentro y se sabe por qué vía"
    );
    lectura.presencias().anotar_salida(&persona);
    lectura.confirmar().await.unwrap();

    assert_eq!(
        fabrica
            .nueva()
            .presencias()
            .via_adentro(&persona)
            .await
            .unwrap(),
        None,
        "al salir deja de estar adentro"
    );
}

pub async fn salir_sin_estar_adentro_no_falla<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let mut uow = fabrica.nueva();
    uow.presencias()
        .anotar_salida(&Identidad::from(&cedula("111111111")));
    assert_eq!(uow.confirmar().await, Ok(()), "la salida es idempotente");
}

/// Dos equipos leen "afuera" al mismo tiempo y ambos registran la entrada:
/// el segundo en confirmar choca, y nada de lo suyo se aplica.
pub async fn dos_equipos_registran_a_la_misma_persona_y_el_segundo_choca<
    F: FabricaUnidadDeTrabajo,
>(
    fabrica: F,
) {
    let persona = Identidad::from(&cedula("111111111"));
    let mut primero = fabrica.nueva();
    let mut segundo = fabrica.nueva();
    assert_eq!(
        primero.presencias().via_adentro(&persona).await.unwrap(),
        None,
        "el primero la ve afuera"
    );
    assert_eq!(
        segundo.presencias().via_adentro(&persona).await.unwrap(),
        None,
        "el segundo también la ve afuera"
    );

    primero.presencias().anotar_entrada(
        &persona,
        Via::Contratista,
        instante("2026-10-09T14:00:00Z"),
    );
    segundo
        .presencias()
        .anotar_entrada(&persona, Via::Correo, instante("2026-10-09T14:00:01Z"));
    segundo.ingresos().guardar(&ingreso(1, None));
    primero.confirmar().await.unwrap();

    assert_eq!(
        segundo.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::PresenciaPersona)),
        "la base no deja a una persona adentro dos veces"
    );
    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura.presencias().via_adentro(&persona).await.unwrap(),
        Some(Via::Contratista),
        "queda la del primero"
    );
    assert_eq!(
        lectura
            .ingresos()
            .obtener(ingreso(1, None).id())
            .await
            .unwrap(),
        None,
        "el ingreso del segundo tampoco se guardó"
    );
}

// --- Gafetes ---

pub async fn guarda_y_lee_un_gafete_con_y_sin_portador<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let tipo = TipoGafete::Contratista;
    let libre = disponible(tipo, 1);
    let perdido_por_contratista = Gafete::restaurar(
        tipo,
        numero(2),
        EstadoGafete::Perdido,
        Some(Portador::Contratista(id_contratista(1))),
    );
    let perdido_por_persona = Gafete::restaurar(
        TipoGafete::Visita,
        numero(3),
        EstadoGafete::Perdido,
        Some(Portador::Persona(cedula("222222222"))),
    );
    let ana_kof = PersonalKof::restaurar(
        PersonalKofId::desde_uuid(Uuid::from_u128(4001)),
        CodigoEmpleado::nuevo("5040017").unwrap(),
        NombrePersona::nuevo("ANA MORA").unwrap(),
        true,
    );
    let perdido_por_personal_kof = Gafete::restaurar(
        TipoGafete::ProvisionalKof,
        numero(4),
        EstadoGafete::Perdido,
        Some(Portador::PersonalKof(ana_kof.id())),
    );
    let mut uow = fabrica.nueva();
    uow.personal_kof().guardar(&ana_kof);
    for gafete in [
        &libre,
        &perdido_por_contratista,
        &perdido_por_persona,
        &perdido_por_personal_kof,
    ] {
        uow.gafetes().agregar(gafete);
    }
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    for esperado in [
        libre,
        perdido_por_contratista,
        perdido_por_persona,
        perdido_por_personal_kof,
    ] {
        assert_eq!(
            lectura
                .gafetes()
                .obtener(esperado.tipo(), esperado.numero())
                .await
                .unwrap(),
            Some(esperado.clone()),
            "vuelve tal cual: {esperado:?}"
        );
    }
    assert_eq!(
        lectura.gafetes().obtener(tipo, numero(99)).await.unwrap(),
        None,
        "un número sin registrar no existe"
    );

    // Actualizar reemplaza el estado y el portador.
    let mut uow = fabrica.nueva();
    uow.gafetes().actualizar(&disponible(tipo, 2));
    uow.confirmar().await.unwrap();
    assert_eq!(
        fabrica
            .nueva()
            .gafetes()
            .obtener(tipo, numero(2))
            .await
            .unwrap(),
        Some(disponible(tipo, 2)),
        "el portador se borra al resolver"
    );
}

pub async fn un_numero_de_gafete_repetido_choca_solo_en_su_tipo<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    let mut uow = fabrica.nueva();
    uow.gafetes()
        .agregar(&disponible(TipoGafete::Contratista, 7));
    uow.confirmar().await.unwrap();

    let mut otro_tipo = fabrica.nueva();
    otro_tipo
        .gafetes()
        .agregar(&disponible(TipoGafete::Proveedor, 7));
    assert_eq!(
        otro_tipo.confirmar().await,
        Ok(()),
        "el mismo número en otro tipo es otro gafete"
    );

    let mut repetido = fabrica.nueva();
    repetido
        .gafetes()
        .agregar(&disponible(TipoGafete::Contratista, 7));
    assert_eq!(
        repetido.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::NumeroGafete)),
        "la base rechaza el número repetido"
    );
}

pub async fn existentes_devuelve_los_del_tipo_de_menor_a_mayor<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    let mut uow = fabrica.nueva();
    for n in [3, 5, 9] {
        uow.gafetes()
            .agregar(&disponible(TipoGafete::Contratista, n));
    }
    uow.gafetes().agregar(&disponible(TipoGafete::Visita, 4));
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    let buscados = [numero(9), numero(4), numero(3), numero(10)];
    assert_eq!(
        lectura
            .gafetes()
            .existentes(TipoGafete::Contratista, &buscados)
            .await
            .unwrap(),
        [numero(3), numero(9)],
        "sólo los de ese tipo, ordenados"
    );
    assert_eq!(
        lectura
            .gafetes()
            .existentes(TipoGafete::Contratista, &[])
            .await
            .unwrap(),
        [],
        "sin números no hay nada que buscar"
    );
}

pub async fn un_gafete_no_se_presta_dos_veces_hasta_que_se_devuelve<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    let tipo = TipoGafete::Contratista;
    let mut primero = fabrica.nueva();
    let mut segundo = fabrica.nueva();
    primero
        .gafetes()
        .anotar_prestamo(tipo, numero(7), instante("2026-10-09T14:00:00Z"));
    segundo
        .gafetes()
        .anotar_prestamo(tipo, numero(7), instante("2026-10-09T14:00:01Z"));
    primero.confirmar().await.unwrap();
    assert_eq!(
        segundo.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::GafetePrestado)),
        "otro equipo ya lo prestó"
    );

    let mut lectura = fabrica.nueva();
    assert!(
        lectura.gafetes().prestado(tipo, numero(7)).await.unwrap(),
        "quedó prestado"
    );
    assert!(
        !lectura
            .gafetes()
            .prestado(TipoGafete::Visita, numero(7))
            .await
            .unwrap(),
        "el préstamo es por tipo"
    );
    lectura.gafetes().anotar_devolucion(tipo, numero(7));
    lectura.confirmar().await.unwrap();

    let mut de_nuevo = fabrica.nueva();
    assert!(
        !de_nuevo.gafetes().prestado(tipo, numero(7)).await.unwrap(),
        "devuelto, ya no está prestado"
    );
    de_nuevo
        .gafetes()
        .anotar_prestamo(tipo, numero(7), instante("2026-10-09T15:00:00Z"));
    assert_eq!(
        de_nuevo.confirmar().await,
        Ok(()),
        "devuelto, se puede prestar otra vez"
    );
}

// --- Ingresos ---

pub async fn guarda_y_lee_un_ingreso_abierto_y_cerrado<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let abierto = ingreso(1, None);
    let a_pie = IngresoContratista::restaurar(IngresoGuardado {
        id: IngresoId::desde_uuid(Uuid::from_u128(5002)),
        contratista: id_contratista(1),
        cedula: cedula("111111111"),
        medio: Medio::APie,
        gafete: None,
        entrada: marca("2026-10-08T08:00:00Z"),
        salida: Some(marca("2026-10-08T17:30:00Z")),
    });
    let mut uow = fabrica.nueva();
    uow.ingresos().guardar(&abierto);
    uow.ingresos().guardar(&a_pie);
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura.ingresos().obtener(abierto.id()).await.unwrap(),
        Some(abierto.clone()),
        "en carro, con gafete y sin salida"
    );
    assert_eq!(
        lectura.ingresos().obtener(a_pie.id()).await.unwrap(),
        Some(a_pie),
        "a pie, sin gafete y con salida"
    );

    // Guardar de nuevo con la salida cierra el mismo ingreso.
    let cerrado = ingreso(1, Some(marca("2026-10-09T22:00:00Z")));
    let mut uow = fabrica.nueva();
    uow.ingresos().guardar(&cerrado);
    uow.confirmar().await.unwrap();
    assert_eq!(
        fabrica
            .nueva()
            .ingresos()
            .obtener(abierto.id())
            .await
            .unwrap(),
        Some(cerrado),
        "el mismo ingreso, ahora con salida"
    );
}

pub async fn abierto_con_gafete_ignora_los_ingresos_cerrados<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let mut uow = fabrica.nueva();
    uow.ingresos()
        .guardar(&ingreso(1, Some(marca("2026-10-09T15:00:00Z"))));
    uow.confirmar().await.unwrap();
    assert_eq!(
        fabrica
            .nueva()
            .ingresos()
            .abierto_con_gafete(numero(7))
            .await
            .unwrap(),
        None,
        "el único ingreso con el gafete 7 ya salió"
    );

    let mut uow = fabrica.nueva();
    uow.ingresos().guardar(&ingreso(2, None));
    uow.confirmar().await.unwrap();
    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura
            .ingresos()
            .abierto_con_gafete(numero(7))
            .await
            .unwrap(),
        Some(ingreso(2, None)),
        "encuentra el que sigue abierto"
    );
    assert_eq!(
        lectura
            .ingresos()
            .abierto_con_gafete(numero(8))
            .await
            .unwrap(),
        None,
        "no hay ingreso abierto con el gafete 8"
    );
}

// --- Reloj ---

pub async fn el_reloj_guarda_el_ultimo_movimiento<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    assert_eq!(
        fabrica.nueva().reloj().ultimo_movimiento().await.unwrap(),
        None,
        "un equipo nuevo no tiene movimientos"
    );
    for en in ["2026-10-09T14:00:00Z", "2026-10-09T14:05:30Z"] {
        let mut uow = fabrica.nueva();
        uow.reloj().anotar_movimiento(instante(en));
        uow.confirmar().await.unwrap();
        assert_eq!(
            fabrica.nueva().reloj().ultimo_movimiento().await.unwrap(),
            Some(instante(en)),
            "queda el último anotado"
        );
    }
}

// --- Todo o nada ---

/// Un choque en cualquier repositorio descarta las escrituras de todos.
pub async fn un_choque_no_aplica_nada_de_ningun_repositorio<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let tipo = TipoGafete::Contratista;
    let mut previo = fabrica.nueva();
    previo
        .gafetes()
        .anotar_prestamo(tipo, numero(7), instante("2026-10-09T13:00:00Z"));
    previo.confirmar().await.unwrap();

    let persona = Identidad::from(&cedula("111111111"));
    let mut uow = fabrica.nueva();
    uow.gafetes().agregar(&disponible(tipo, 1));
    uow.presencias()
        .anotar_entrada(&persona, Via::Contratista, instante("2026-10-09T14:00:00Z"));
    uow.reloj()
        .anotar_movimiento(instante("2026-10-09T14:00:00Z"));
    uow.gafetes()
        .anotar_prestamo(tipo, numero(7), instante("2026-10-09T14:00:00Z"));
    assert_eq!(
        uow.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::GafetePrestado)),
        "el préstamo choca al final de la transacción"
    );

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura.gafetes().obtener(tipo, numero(1)).await.unwrap(),
        None,
        "tampoco se agregó el gafete nuevo"
    );
    assert_eq!(
        lectura.presencias().via_adentro(&persona).await.unwrap(),
        None,
        "tampoco quedó adentro"
    );
    assert_eq!(
        lectura.reloj().ultimo_movimiento().await.unwrap(),
        None,
        "tampoco se anotó el reloj"
    );
}
