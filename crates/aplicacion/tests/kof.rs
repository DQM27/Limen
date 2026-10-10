//! Pruebas de los casos de uso del personal KOF contra los dobles en
//! memoria.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use limen_aplicacion::casos_de_uso::gafetes::RegistrarGafetes;
    use limen_aplicacion::casos_de_uso::kof::{
        DevolverGafeteKof, EditarPersonalKof, EntregarGafeteKof, RegistrarPersonalKof,
    };
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::{
        AccionAuditada, ErrorPersistencia, RegistroAuditado, Restriccion,
    };
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, TipoGafete};
    use limen_dominio::personal_kof::{CodigoEmpleado, ErrorPersonalKof, PersonalKofId};
    use limen_dominio::presencia::{Identidad, Via};
    use limen_dominio::prestamo_kof::{ErrorDevolucionKof, ErrorPrestamoKof, PrestamoKofId};
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales, RelojFijo};
    use uuid::Uuid;

    const ENTREGA: &str = "2026-10-09T08:00:00Z";
    const DEVOLUCION: &str = "2026-10-09T17:00:00Z";

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    fn reloj_a(texto: &str) -> RelojFijo {
        RelojFijo::new(
            instante(texto),
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        )
    }

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    fn numero(n: u32) -> NumeroGafete {
        NumeroGafete::nuevo(n).unwrap()
    }

    type Registrar = RegistrarPersonalKof<AlmacenMemoria, RelojFijo, IdsSecuenciales>;
    type Entregar = EntregarGafeteKof<AlmacenMemoria, RelojFijo, IdsSecuenciales>;

    fn registrar(almacen: &AlmacenMemoria) -> Registrar {
        RegistrarPersonalKof::new(almacen.clone(), reloj_a(ENTREGA), almacen.ids())
    }

    fn entregar(almacen: &AlmacenMemoria) -> Entregar {
        EntregarGafeteKof::new(almacen.clone(), reloj_a(ENTREGA), almacen.ids())
    }

    fn devolver(
        almacen: &AlmacenMemoria,
    ) -> DevolverGafeteKof<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        DevolverGafeteKof::new(almacen.clone(), reloj_a(DEVOLUCION), almacen.ids())
    }

    /// Almacén con los gafetes provisionales KOF 1 a 5 y dos personas.
    async fn preparado() -> (AlmacenMemoria, PersonalKofId, PersonalKofId) {
        let almacen = AlmacenMemoria::new();
        RegistrarGafetes::new(almacen.clone(), reloj_a(ENTREGA), almacen.ids())
            .ejecutar(&sesion(), TipoGafete::ProvisionalKof, 1, 5)
            .await
            .unwrap();
        let ana = registrar(&almacen)
            .ejecutar(&sesion(), "5040017", "ana mora")
            .await
            .unwrap();
        let beto = registrar(&almacen)
            .ejecutar(&sesion(), "5040018", "beto solís")
            .await
            .unwrap();
        (almacen, ana, beto)
    }

    // --- Catálogo ---

    #[tokio::test]
    async fn registra_a_una_persona_y_audita_el_alta() {
        let almacen = AlmacenMemoria::new();
        let id = registrar(&almacen)
            .ejecutar(&sesion(), " 5040017 ", "ana mora")
            .await
            .unwrap();
        let personal = almacen.personal_kof();
        assert_eq!(personal.len(), 1, "quedó guardada");
        assert_eq!(personal[0].codigo().as_str(), "5040017");
        assert_eq!(personal[0].nombre().as_str(), "ANA MORA");
        assert!(personal[0].activo(), "nace activa");
        let auditoria = almacen.auditoria();
        assert_eq!(auditoria.len(), 1, "una entrada");
        assert_eq!(auditoria[0].registro, RegistroAuditado::PersonalKof(id));
        assert_eq!(auditoria[0].accion, AccionAuditada::Alta);
        assert_eq!(auditoria[0].cambios.len(), 3, "código, nombre y activo");
    }

    #[tokio::test]
    async fn el_codigo_no_se_repite_ni_aunque_otro_equipo_gane() {
        let (almacen, _, _) = preparado().await;
        assert_eq!(
            registrar(&almacen)
                .ejecutar(&sesion(), "5040017", "otra")
                .await,
            Err(ErrorCaso::Negocio(ErrorPersonalKof::CodigoRepetido))
        );
        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::CodigoEmpleado));
        assert_eq!(
            registrar(&almacen)
                .ejecutar(&sesion(), "5040099", "otra")
                .await,
            Err(ErrorCaso::Negocio(ErrorPersonalKof::CodigoRepetido)),
            "el choque de la base es el mismo error"
        );
    }

    #[tokio::test]
    async fn el_codigo_y_el_nombre_se_validan_antes_de_leer_la_base() {
        let almacen = AlmacenMemoria::new();
        almacen.fallar_lecturas(ErrorPersistencia::Tecnica("no debería leer".into()));
        assert_eq!(
            registrar(&almacen).ejecutar(&sesion(), "12", "ANA").await,
            Err(ErrorCaso::Negocio(ErrorPersonalKof::CodigoInvalido))
        );
    }

    #[tokio::test]
    async fn editar_audita_solo_lo_que_cambio_y_no_escribe_si_nada_cambio() {
        let (almacen, ana, _) = preparado().await;
        let editar = EditarPersonalKof::new(almacen.clone(), reloj_a(ENTREGA), almacen.ids());
        let cambios = editar
            .ejecutar(&sesion(), ana, "ana mora", false)
            .await
            .unwrap();
        assert_eq!(cambios.len(), 1, "sólo cambió activo");
        let ultima = almacen.auditoria().pop().unwrap();
        assert_eq!(ultima.accion, AccionAuditada::Edicion);
        assert_eq!(ultima.registro, RegistroAuditado::PersonalKof(ana));

        let antes = almacen.confirmaciones();
        assert_eq!(
            editar.ejecutar(&sesion(), ana, "ANA MORA", false).await,
            Ok(Vec::new())
        );
        assert_eq!(almacen.confirmaciones(), antes, "sin cambios no escribe");
        assert_eq!(
            editar
                .ejecutar(
                    &sesion(),
                    PersonalKofId::desde_uuid(Uuid::from_u128(1)),
                    "X",
                    true
                )
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    // --- Préstamo ---

    #[tokio::test]
    async fn la_entrega_guarda_el_prestamo_y_marca_el_gafete_prestado() {
        let (almacen, ana, _) = preparado().await;
        let id = entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        let prestamos = almacen.prestamos_kof();
        assert_eq!(prestamos.len(), 1, "un préstamo");
        assert_eq!(prestamos[0].id(), id);
        assert_eq!(prestamos[0].personal(), ana);
        assert_eq!(prestamos[0].gafete(), numero(3));
        assert_eq!(prestamos[0].entrega().operador, sesion().operador());
        assert!(
            almacen.prestado(TipoGafete::ProvisionalKof, numero(3)),
            "el gafete está prestado"
        );
    }

    #[tokio::test]
    async fn una_persona_inactiva_no_recibe_gafete() {
        let (almacen, ana, _) = preparado().await;
        EditarPersonalKof::new(almacen.clone(), reloj_a(ENTREGA), almacen.ids())
            .ejecutar(&sesion(), ana, "ANA MORA", false)
            .await
            .unwrap();
        assert_eq!(
            entregar(&almacen).ejecutar(&sesion(), ana, 3).await,
            Err(ErrorCaso::Negocio(ErrorPrestamoKof::PersonalInactivo))
        );
        assert!(almacen.prestamos_kof().is_empty(), "no se prestó nada");
    }

    #[tokio::test]
    async fn la_persona_debe_existir() {
        let (almacen, _, _) = preparado().await;
        assert_eq!(
            entregar(&almacen)
                .ejecutar(&sesion(), PersonalKofId::desde_uuid(Uuid::from_u128(1)), 3)
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn una_persona_no_tiene_dos_provisionales() {
        let (almacen, ana, _) = preparado().await;
        entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        assert_eq!(
            entregar(&almacen).ejecutar(&sesion(), ana, 4).await,
            Err(ErrorCaso::Negocio(ErrorPrestamoKof::YaTienePrestamo))
        );
        assert!(
            !almacen.prestado(TipoGafete::ProvisionalKof, numero(4)),
            "el segundo gafete quedó libre"
        );
    }

    #[tokio::test]
    async fn un_gafete_no_se_presta_a_dos_personas() {
        let (almacen, ana, beto) = preparado().await;
        entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        assert_eq!(
            entregar(&almacen).ejecutar(&sesion(), beto, 3).await,
            Err(ErrorCaso::Negocio(ErrorPrestamoKof::Gafete(
                ErrorPrestamoGafete::Prestado
            )))
        );
    }

    #[tokio::test]
    async fn el_gafete_debe_ser_provisional_kof() {
        let (almacen, ana, _) = preparado().await;
        RegistrarGafetes::new(almacen.clone(), reloj_a(ENTREGA), almacen.ids())
            .ejecutar(&sesion(), TipoGafete::Visita, 50, 50)
            .await
            .unwrap();
        for gafete in [50, 99, 0] {
            assert_eq!(
                entregar(&almacen).ejecutar(&sesion(), ana, gafete).await,
                Err(ErrorCaso::Negocio(ErrorPrestamoKof::Gafete(
                    ErrorPrestamoGafete::NoRegistrado
                ))),
                "el {gafete} no está en el inventario KOF"
            );
        }
    }

    #[tokio::test]
    async fn los_choques_con_otro_equipo_llegan_como_errores_de_negocio() {
        let (almacen, ana, _) = preparado().await;
        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(
            Restriccion::PersonalKofConPrestamo,
        ));
        assert_eq!(
            entregar(&almacen).ejecutar(&sesion(), ana, 3).await,
            Err(ErrorCaso::Negocio(ErrorPrestamoKof::YaTienePrestamo))
        );
        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::GafetePrestado));
        assert_eq!(
            entregar(&almacen).ejecutar(&sesion(), ana, 3).await,
            Err(ErrorCaso::Negocio(ErrorPrestamoKof::Gafete(
                ErrorPrestamoGafete::Prestado
            )))
        );
        assert!(almacen.prestamos_kof().is_empty(), "nada se aplicó");
    }

    // --- Estar adentro ---

    fn identidad(codigo: &str) -> Identidad {
        Identidad::from(&CodigoEmpleado::nuevo(codigo).unwrap())
    }

    #[tokio::test]
    async fn entregar_el_gafete_es_la_entrada_y_devolverlo_la_salida() {
        let (almacen, ana, _) = preparado().await;
        assert_eq!(almacen.via_adentro(identidad("5040017")), None, "afuera");

        entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        assert_eq!(
            almacen.via_adentro(identidad("5040017")),
            Some(Via::Kof),
            "con el gafete provisional está adentro"
        );
        assert_eq!(
            almacen.ultimo_movimiento(),
            Some(instante(ENTREGA)),
            "la entrega es un movimiento"
        );

        devolver(&almacen).por_gafete(&sesion(), 3).await.unwrap();
        assert_eq!(
            almacen.via_adentro(identidad("5040017")),
            None,
            "al devolverlo sale"
        );
        assert_eq!(
            almacen.ultimo_movimiento(),
            Some(instante(DEVOLUCION)),
            "la devolución también"
        );
    }

    #[tokio::test]
    async fn aparece_en_la_lista_de_quienes_estan_adentro() {
        use limen_aplicacion::casos_de_uso::consultas::QuienesEstanAdentro;
        use limen_aplicacion::puertos::IngresoAbierto;

        let (almacen, ana, _) = preparado().await;
        let id = entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        let adentro = QuienesEstanAdentro::new(almacen.clone())
            .ejecutar()
            .await
            .unwrap();
        assert_eq!(adentro.len(), 1, "una persona");
        assert_eq!(adentro[0].ingreso, IngresoAbierto::Kof(id));
        assert_eq!(adentro[0].identidad.to_string(), "5040017");
        assert_eq!(adentro[0].nombre.as_str(), "ANA MORA");
        assert_eq!(adentro[0].procedencia, "Personal KOF");

        devolver(&almacen).ejecutar(&sesion(), id).await.unwrap();
        assert!(
            QuienesEstanAdentro::new(almacen.clone())
                .ejecutar()
                .await
                .unwrap()
                .is_empty(),
            "ya no está"
        );
    }

    #[tokio::test]
    async fn con_el_reloj_atrasado_se_entrega_igual_y_la_hora_queda_marcada() {
        let (almacen, ana, _) = preparado().await;
        almacen.fijar_ultimo_movimiento(instante("2026-10-09T09:00:00Z"));
        entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        assert_eq!(almacen.prestamos_kof().len(), 1, "se prestó");
        assert!(
            almacen
                .hechos()
                .last()
                .is_some_and(|hecho| !hecho.hora().confiable),
            "la hora queda marcada para revisarla"
        );
    }

    #[tokio::test]
    async fn si_otro_equipo_ya_lo_dejo_entrar_es_el_mismo_error_de_negocio() {
        let (almacen, ana, _) = preparado().await;
        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(
            Restriccion::PresenciaPersona,
        ));
        assert_eq!(
            entregar(&almacen).ejecutar(&sesion(), ana, 3).await,
            Err(ErrorCaso::Negocio(ErrorPrestamoKof::YaTienePrestamo)),
            "tener el provisional y estar adentro es lo mismo"
        );
    }

    // --- Devolución ---

    #[tokio::test]
    async fn la_devolucion_libera_el_gafete_y_a_la_persona() {
        let (almacen, ana, beto) = preparado().await;
        let id = entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        devolver(&almacen).ejecutar(&sesion(), id).await.unwrap();

        let prestamo = &almacen.prestamos_kof()[0];
        assert!(!prestamo.esta_abierto(), "devuelto");
        assert_eq!(prestamo.devolucion().unwrap().en, instante(DEVOLUCION));
        assert!(
            !almacen.prestado(TipoGafete::ProvisionalKof, numero(3)),
            "el gafete quedó libre"
        );
        // Ana puede recibir otro y Beto puede recibir el 3 (más tarde: el
        // reloj no retrocede).
        let despues = || {
            EntregarGafeteKof::new(
                almacen.clone(),
                reloj_a("2026-10-09T18:00:00Z"),
                almacen.ids(),
            )
        };
        despues().ejecutar(&sesion(), ana, 4).await.unwrap();
        despues().ejecutar(&sesion(), beto, 3).await.unwrap();

        assert_eq!(
            devolver(&almacen).ejecutar(&sesion(), id).await,
            Err(ErrorCaso::Negocio(ErrorDevolucionKof::YaDevuelto))
        );
    }

    #[tokio::test]
    async fn la_devolucion_por_gafete_cierra_el_prestamo_abierto() {
        let (almacen, ana, _) = preparado().await;
        entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        devolver(&almacen).por_gafete(&sesion(), 3).await.unwrap();
        assert!(!almacen.prestamos_kof()[0].esta_abierto(), "cerrado");
        assert_eq!(
            devolver(&almacen).por_gafete(&sesion(), 3).await,
            Err(ErrorCaso::NoEncontrado),
            "ya no hay un préstamo abierto con ese gafete"
        );
        assert_eq!(
            devolver(&almacen).por_gafete(&sesion(), 0).await,
            Err(ErrorCaso::NoEncontrado)
        );
        assert_eq!(
            devolver(&almacen)
                .ejecutar(&sesion(), PrestamoKofId::desde_uuid(Uuid::from_u128(1)))
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn devolver_con_el_reloj_atrasado_no_queda_antes_de_la_entrega() {
        let (almacen, ana, _) = preparado().await;
        let id = entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        DevolverGafeteKof::new(
            almacen.clone(),
            reloj_a("2026-10-09T07:00:00Z"),
            almacen.ids(),
        )
        .ejecutar(&sesion(), id)
        .await
        .unwrap();
        assert!(
            !almacen.prestado(TipoGafete::ProvisionalKof, numero(3)),
            "se devolvió"
        );
        assert!(
            almacen.prestamos_kof().iter().all(|prestamo| prestamo
                .devolucion()
                .is_some_and(|devolucion| devolucion.en >= prestamo.entrega().en)),
            "con la hora de la entrega, nunca antes"
        );
        assert!(
            almacen
                .hechos()
                .last()
                .is_some_and(|hecho| !hecho.hora().confiable),
            "la hora queda marcada para revisarla"
        );
    }

    // --- Hechos ---

    /// Los hechos guardados son exactamente la entrada y la salida de
    /// `registro`, en ese orden, por la vía indicada.
    fn entrada_y_salida(almacen: &AlmacenMemoria, via: Via, registro: Uuid) {
        let hechos = almacen.hechos();
        let [entro, salio] = hechos.as_slice() else {
            panic!("se esperaban dos hechos: {hechos:?}");
        };
        for hecho in [entro, salio] {
            assert_eq!(hecho.via(), via, "la vía del hecho");
            assert_eq!(hecho.registro(), registro, "el registro del hecho");
            assert_eq!(hecho.marca().operador, sesion().operador(), "quién");
        }
        assert!(entro.es_entrada(), "primero la entrada");
        assert!(!salio.es_entrada(), "después la salida");
        assert!(entro.id() < salio.id(), "el ID ordena los hechos");
    }

    #[tokio::test]
    async fn la_entrega_y_la_devolucion_dejan_cada_una_su_hecho() {
        let (almacen, ana, _) = preparado().await;
        let id = entregar(&almacen)
            .ejecutar(&sesion(), ana, 3)
            .await
            .unwrap();
        let hechos_previos = almacen.hechos().len();
        assert_eq!(
            hechos_previos, 1,
            "registrar personas no deja hechos; la entrega sí"
        );
        devolver(&almacen).ejecutar(&sesion(), id).await.unwrap();
        entrada_y_salida(&almacen, Via::Kof, id.uuid());
    }
}
