//! Contrato de los repositorios del personal KOF y de los préstamos de
//! gafete provisional.

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{
    ErrorPersistencia, FabricaUnidadDeTrabajo, RepositorioPersonalKof, RepositorioPrestamosKof,
    Restriccion, UnidadDeTrabajo,
};
use limen_dominio::gafete::NumeroGafete;
use limen_dominio::movimiento::Marca;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::personal_kof::{CodigoEmpleado, PersonalKof, PersonalKofId};
use limen_dominio::prestamo_kof::{PrestamoKof, PrestamoKofGuardado, PrestamoKofId};
use uuid::Uuid;

// --- Datos de ejemplo ---

fn id_persona(n: u128) -> PersonalKofId {
    PersonalKofId::desde_uuid(Uuid::from_u128(4000 + n))
}

fn persona(n: u128, codigo: &str, activa: bool) -> PersonalKof {
    PersonalKof::restaurar(
        id_persona(n),
        CodigoEmpleado::nuevo(codigo).unwrap(),
        NombrePersona::nuevo("ANA MORA").unwrap(),
        activa,
    )
}

fn marca(texto: &str) -> Marca {
    let en: DateTime<Utc> = texto.parse().unwrap();
    Marca {
        en,
        operador: OperadorId::desde_uuid(Uuid::from_u128(900)),
    }
}

fn prestamo(n: u128, persona_n: u128, gafete: u32, devolucion: Option<Marca>) -> PrestamoKof {
    PrestamoKof::restaurar(PrestamoKofGuardado {
        id: PrestamoKofId::desde_uuid(Uuid::from_u128(8000 + n)),
        personal: id_persona(persona_n),
        // La persona 1 es el código 5040017, la 2 el 5040018…
        codigo: CodigoEmpleado::nuevo(&(5_040_016 + persona_n).to_string()).unwrap(),
        nombre: NombrePersona::nuevo("ANA MORA").unwrap(),
        gafete: NumeroGafete::nuevo(gafete).unwrap(),
        entrega: marca("2026-10-09T08:00:00Z"),
        devolucion,
    })
}

async fn sembrar_personal<F: FabricaUnidadDeTrabajo>(fabrica: &F, personas: &[PersonalKof]) {
    let mut uow = fabrica.nueva();
    for persona in personas {
        uow.personal_kof().guardar(persona);
    }
    uow.confirmar().await.unwrap();
}

// --- Personal ---

pub async fn guarda_y_lee_al_personal_kof<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    let activa = persona(1, "5040017", true);
    let inactiva = persona(2, "5040018", false);
    sembrar_personal(&fabrica, &[activa.clone(), inactiva.clone()]).await;

    let mut lectura = fabrica.nueva();
    for esperada in [activa, inactiva] {
        assert_eq!(
            lectura.personal_kof().obtener(esperada.id()).await.unwrap(),
            Some(esperada.clone()),
            "vuelve tal cual: {esperada:?}"
        );
    }
    assert_eq!(
        lectura.personal_kof().obtener(id_persona(9)).await.unwrap(),
        None,
        "lo inexistente no se encuentra"
    );

    // Guardar de nuevo actualiza.
    let mut uow = fabrica.nueva();
    uow.personal_kof().guardar(&persona(1, "5040017", false));
    uow.confirmar().await.unwrap();
    assert_eq!(
        fabrica
            .nueva()
            .personal_kof()
            .obtener(id_persona(1))
            .await
            .unwrap(),
        Some(persona(1, "5040017", false)),
        "ahora inactiva"
    );
}

pub async fn el_codigo_de_empleado_repetido_choca<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_personal(&fabrica, &[persona(1, "5040017", true)]).await;
    let mut repetido = fabrica.nueva();
    repetido
        .personal_kof()
        .guardar(&persona(2, "5040017", true));
    assert_eq!(
        repetido.confirmar().await,
        Err(ErrorPersistencia::Conflicto(Restriccion::CodigoEmpleado)),
        "la base rechaza el código repetido"
    );
}

pub async fn codigo_en_uso_excluye_a_la_propia_persona<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_personal(&fabrica, &[persona(1, "5040017", true)]).await;
    let mut uow = fabrica.nueva();
    let codigo = CodigoEmpleado::nuevo("5040017").unwrap();
    assert!(
        !uow.personal_kof()
            .codigo_en_uso(&codigo, Some(id_persona(1)))
            .await
            .unwrap(),
        "su propio código no cuenta"
    );
    assert!(
        uow.personal_kof()
            .codigo_en_uso(&codigo, Some(id_persona(2)))
            .await
            .unwrap(),
        "para otra persona sí está en uso"
    );
    assert!(
        !uow.personal_kof()
            .codigo_en_uso(&CodigoEmpleado::nuevo("5040099").unwrap(), None)
            .await
            .unwrap(),
        "un código libre"
    );
}

// --- Préstamos ---

pub async fn guarda_y_lee_un_prestamo_kof<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_personal(
        &fabrica,
        &[persona(1, "5040017", true), persona(2, "5040018", true)],
    )
    .await;
    let abierto = prestamo(1, 1, 3, None);
    let mut uow = fabrica.nueva();
    uow.prestamos_kof().anotar_entrega(&abierto);
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura.prestamos_kof().obtener(abierto.id()).await.unwrap(),
        Some(abierto.clone()),
        "vuelve tal cual"
    );
    assert!(
        lectura
            .prestamos_kof()
            .tiene_abierto(id_persona(1))
            .await
            .unwrap(),
        "Ana tiene un provisional"
    );
    assert!(
        !lectura
            .prestamos_kof()
            .tiene_abierto(id_persona(2))
            .await
            .unwrap(),
        "Beto no"
    );
    assert_eq!(
        lectura
            .prestamos_kof()
            .abierto_con_gafete(NumeroGafete::nuevo(3).unwrap())
            .await
            .unwrap(),
        Some(abierto.clone()),
        "se encuentra por el gafete"
    );
    assert_eq!(
        lectura
            .prestamos_kof()
            .abierto_con_gafete(NumeroGafete::nuevo(4).unwrap())
            .await
            .unwrap(),
        None,
        "nadie tiene el 4"
    );
    assert_eq!(
        lectura
            .prestamos_kof()
            .obtener(PrestamoKofId::desde_uuid(Uuid::from_u128(1)))
            .await
            .unwrap(),
        None,
        "lo inexistente no se encuentra"
    );
}

pub async fn la_devolucion_libera_a_la_persona_y_al_gafete<F: FabricaUnidadDeTrabajo>(fabrica: F) {
    sembrar_personal(&fabrica, &[persona(1, "5040017", true)]).await;
    let mut uow = fabrica.nueva();
    uow.prestamos_kof().anotar_entrega(&prestamo(1, 1, 3, None));
    uow.confirmar().await.unwrap();

    let devuelto = prestamo(1, 1, 3, Some(marca("2026-10-09T17:00:00Z")));
    let mut uow = fabrica.nueva();
    uow.prestamos_kof().anotar_devolucion(&devuelto);
    uow.confirmar().await.unwrap();

    let mut lectura = fabrica.nueva();
    assert_eq!(
        lectura
            .prestamos_kof()
            .obtener(devuelto.id())
            .await
            .unwrap(),
        Some(devuelto),
        "quedó cerrado"
    );
    assert!(
        !lectura
            .prestamos_kof()
            .tiene_abierto(id_persona(1))
            .await
            .unwrap(),
        "Ana ya no tiene provisional"
    );
    assert_eq!(
        lectura
            .prestamos_kof()
            .abierto_con_gafete(NumeroGafete::nuevo(3).unwrap())
            .await
            .unwrap(),
        None,
        "el gafete ya no está abierto"
    );

    // Puede recibir otro.
    let mut otro = fabrica.nueva();
    otro.prestamos_kof()
        .anotar_entrega(&prestamo(2, 1, 4, None));
    assert_eq!(
        otro.confirmar().await,
        Ok(()),
        "devuelto, puede recibir otro"
    );
}

pub async fn una_persona_no_recibe_dos_provisionales_a_la_vez<F: FabricaUnidadDeTrabajo>(
    fabrica: F,
) {
    sembrar_personal(&fabrica, &[persona(1, "5040017", true)]).await;
    let mut primero = fabrica.nueva();
    let mut segundo = fabrica.nueva();
    primero
        .prestamos_kof()
        .anotar_entrega(&prestamo(1, 1, 3, None));
    segundo
        .prestamos_kof()
        .anotar_entrega(&prestamo(2, 1, 4, None));
    primero.confirmar().await.unwrap();
    assert_eq!(
        segundo.confirmar().await,
        Err(ErrorPersistencia::Conflicto(
            Restriccion::PersonalKofConPrestamo
        )),
        "otro equipo ya le dio uno"
    );
    assert_eq!(
        fabrica
            .nueva()
            .prestamos_kof()
            .obtener(prestamo(2, 1, 4, None).id())
            .await
            .unwrap(),
        None,
        "el segundo préstamo no quedó"
    );
}
