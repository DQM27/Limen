//! Pruebas de los casos de uso de ingresos, salidas y gafetes contra los
//! dobles en memoria.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use limen_aplicacion::casos_de_uso::contratistas::{ComandoContratista, RegistrarContratista};
    use limen_aplicacion::casos_de_uso::gafetes::{CambiarGafete, CambioGafete, RegistrarGafetes};
    use limen_aplicacion::casos_de_uso::ingresos::{
        ComandoEntrada, GafeteElegido, PrepararIngreso, RegistrarEntrada, RegistrarSalida,
    };
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::{
        AccionAuditada, ErrorPersistencia, RegistroAuditado, Restriccion,
    };
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::acceso::{MotivoDenegacion, ResultadoAcceso};
    use limen_dominio::cedula::Cedula;
    use limen_dominio::contratista::ContratistaId;
    use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
    use limen_dominio::gafete::{
        ErrorGafete, ErrorPrestamoGafete, EstadoGafete, NumeroGafete, Portador, Resolucion,
        TipoGafete,
    };
    use limen_dominio::hecho::Suceso;
    use limen_dominio::ingreso_contratista::{EntregaGafete, ErrorIngreso};
    use limen_dominio::medio::{ErrorMedio, TipoMedio};
    use limen_dominio::movimiento::ErrorSalida;
    use limen_dominio::presencia::{Via, YaEstaAdentro};
    use limen_dominio::reloj::RelojAtrasado;
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales, RelojFijo};
    use uuid::Uuid;

    // --- Preparación ---

    const ENTRADA: &str = "2026-10-09T14:00:00Z";
    const SALIDA: &str = "2026-10-09T22:00:00Z";

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    /// Reloj detenido en `texto`; el día de Costa Rica es 2026-10-09.
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

    fn entrada(
        almacen: &AlmacenMemoria,
    ) -> RegistrarEntrada<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        RegistrarEntrada::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
    }

    fn salida(
        almacen: &AlmacenMemoria,
    ) -> RegistrarSalida<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        RegistrarSalida::new(almacen.clone(), reloj_a(SALIDA), almacen.ids())
    }

    fn gafetes(
        almacen: &AlmacenMemoria,
    ) -> RegistrarGafetes<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        RegistrarGafetes::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
    }

    fn cambiar(
        almacen: &AlmacenMemoria,
    ) -> CambiarGafete<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        CambiarGafete::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
    }

    /// Almacén con la empresa ACME y los gafetes de contratista 1 a 30.
    async fn preparado() -> AlmacenMemoria {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_empresa(Empresa::restaurar(
            EmpresaId::desde_uuid(Uuid::from_u128(500)),
            NombreEmpresa::nuevo("ACME").unwrap(),
        ));
        gafetes(&almacen)
            .ejecutar(&sesion(), TipoGafete::Contratista, 1, 30)
            .await
            .unwrap();
        almacen
    }

    async fn con_contratista(
        almacen: &AlmacenMemoria,
        cedula: &str,
        tipo: TipoIngreso,
        vence: &str,
        tiene_acceso: bool,
    ) -> ContratistaId {
        let comando = ComandoContratista {
            cedula: cedula.into(),
            nombre: "ANA SOLANO".into(),
            empresa: EmpresaId::desde_uuid(Uuid::from_u128(500)),
            tipo_ingreso: tipo,
            fecha_vencimiento_praind: vence.parse().unwrap(),
            tiene_acceso,
        };
        RegistrarContratista::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
            .ejecutar(&sesion(), &comando)
            .await
            .unwrap()
    }

    async fn praind(almacen: &AlmacenMemoria, cedula: &str) -> ContratistaId {
        con_contratista(almacen, cedula, TipoIngreso::Praind, "2027-01-01", true).await
    }

    /// A pie, con ese gafete o, si no se indica número, "sin gafete" (S/G)
    /// marcado a propósito.
    fn a_pie(contratista: ContratistaId, gafete: Option<u32>) -> ComandoEntrada {
        ComandoEntrada {
            contratista,
            medio: TipoMedio::APie,
            placa: None,
            gafete: Some(gafete.map_or(GafeteElegido::SinGafete, GafeteElegido::Numero)),
        }
    }

    fn cedula(texto: &str) -> Cedula {
        Cedula::normalizar(texto).unwrap()
    }

    // --- Entrada ---

    #[tokio::test]
    async fn la_entrada_guarda_el_ingreso_la_presencia_el_gafete_y_el_reloj() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let resultado = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, Some(25)))
            .await
            .unwrap();
        assert_eq!(resultado.acceso, ResultadoAcceso::Permitido);

        let ingresos = almacen.ingresos();
        assert_eq!(ingresos.len(), 1, "un ingreso");
        assert_eq!(ingresos[0].id(), resultado.ingreso);
        assert!(ingresos[0].esta_abierto(), "sigue adentro");
        assert_eq!(ingresos[0].entrada().operador, sesion().operador());
        assert_eq!(
            almacen.via_adentro(&cedula("111111111")),
            Some(Via::Contratista),
            "la presencia quedó anotada"
        );
        assert!(
            almacen.prestado(TipoGafete::Contratista, numero(25)),
            "el gafete quedó prestado"
        );
        assert_eq!(
            almacen.ultimo_movimiento(),
            Some(instante(ENTRADA)),
            "reloj"
        );
    }

    #[tokio::test]
    async fn se_puede_entrar_sin_gafete() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, None))
            .await
            .unwrap();
        assert_eq!(almacen.ingresos()[0].gafete(), None, "S/G");
    }

    #[tokio::test]
    async fn avisa_si_el_praind_esta_por_vencer() {
        let almacen = preparado().await;
        let id = con_contratista(
            &almacen,
            "111111111",
            TipoIngreso::InHouse,
            "2026-10-20",
            true,
        )
        .await;
        let resultado = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, None))
            .await
            .unwrap();
        assert_eq!(
            resultado.acceso,
            ResultadoAcceso::PermitidoConAdvertencia {
                dias_para_vencer: 11
            }
        );
    }

    #[tokio::test]
    async fn un_contratista_que_no_existe_es_no_encontrado() {
        let almacen = preparado().await;
        let fantasma = ContratistaId::desde_uuid(Uuid::from_u128(404));
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(fantasma, None))
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn no_entra_dos_veces() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, Some(1)))
            .await
            .unwrap();
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(id, Some(2)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::YaEstaAdentro(
                YaEstaAdentro(Via::Contratista)
            )))
        );
        assert_eq!(almacen.ingresos().len(), 1, "sigue habiendo uno solo");
        assert!(
            !almacen.prestado(TipoGafete::Contratista, numero(2)),
            "el segundo gafete no se prestó"
        );
    }

    #[tokio::test]
    async fn no_entra_si_ya_esta_adentro_por_otra_via() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        almacen.marcar_adentro(&cedula("111111111"), Via::Proveedor);
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(id, Some(1)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::YaEstaAdentro(
                YaEstaAdentro(Via::Proveedor)
            )))
        );
        assert!(almacen.ingresos().is_empty(), "no se registró nada");
    }

    #[tokio::test]
    async fn sin_acceso_no_entra_y_no_se_escribe_nada() {
        let almacen = preparado().await;
        let id = con_contratista(
            &almacen,
            "111111111",
            TipoIngreso::Praind,
            "2027-01-01",
            false,
        )
        .await;
        let confirmaciones = almacen.confirmaciones();
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(id, Some(1)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::AccesoDenegado(
                MotivoDenegacion::SinAcceso
            )))
        );
        assert_eq!(almacen.confirmaciones(), confirmaciones, "nada confirmado");
        assert_eq!(almacen.via_adentro(&cedula("111111111")), None);
    }

    #[tokio::test]
    async fn en_vehiculo_sin_placa_se_rechaza() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let sin_placa = ComandoEntrada {
            medio: TipoMedio::Vehiculo,
            ..a_pie(id, None)
        };
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &sin_placa).await,
            Err(ErrorCaso::Negocio(ErrorIngreso::Medio(
                ErrorMedio::PlacaRequerida
            )))
        );
    }

    #[tokio::test]
    async fn con_el_reloj_atrasado_no_entra() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        almacen.fijar_ultimo_movimiento(instante("2026-10-09T15:00:00Z"));
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(id, None))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::Reloj(RelojAtrasado)))
        );
    }

    #[tokio::test]
    async fn el_gafete_debe_existir_y_estar_libre() {
        let almacen = preparado().await;
        let ana = praind(&almacen, "111111111").await;
        let beto = praind(&almacen, "222222222").await;
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(ana, Some(99)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::Gafete(
                ErrorPrestamoGafete::NoRegistrado
            ))),
            "el 99 no está en el catálogo"
        );
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(ana, Some(0)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::Gafete(
                ErrorPrestamoGafete::NoRegistrado
            ))),
            "el 0 no existe en ningún catálogo"
        );
        entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(ana, Some(5)))
            .await
            .unwrap();
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(beto, Some(5)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::Gafete(
                ErrorPrestamoGafete::Prestado
            ))),
            "el 5 ya lo tiene Ana"
        );
    }

    #[tokio::test]
    async fn un_gafete_perdido_no_se_presta() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        cambiar(&almacen)
            .ejecutar(
                &sesion(),
                TipoGafete::Contratista,
                7,
                CambioGafete::MarcarPerdido(Portador::Contratista(id)),
            )
            .await
            .unwrap();
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(id, Some(7)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::Gafete(
                ErrorPrestamoGafete::NoDisponible(EstadoGafete::Perdido)
            )))
        );
    }

    #[tokio::test]
    async fn los_choques_con_otro_equipo_llegan_como_errores_de_negocio() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(
            Restriccion::PresenciaPersona,
        ));
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(id, None))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::YaEstaAdentro(
                YaEstaAdentro(Via::Contratista)
            ))),
            "otro equipo la registró primero"
        );
        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::GafetePrestado));
        assert_eq!(
            entrada(&almacen)
                .ejecutar(&sesion(), &a_pie(id, Some(3)))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::Gafete(
                ErrorPrestamoGafete::Prestado
            ))),
            "otro equipo prestó ese gafete primero"
        );
    }

    // --- Salida ---

    #[tokio::test]
    async fn la_salida_cierra_el_ingreso_y_libera_presencia_y_gafete() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let ingreso = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, Some(25)))
            .await
            .unwrap()
            .ingreso;
        salida(&almacen).ejecutar(&sesion(), ingreso).await.unwrap();

        let cerrado = &almacen.ingresos()[0];
        assert_eq!(cerrado.salida().map(|m| m.en), Some(instante(SALIDA)));
        assert_eq!(almacen.via_adentro(&cedula("111111111")), None, "ya salió");
        assert!(
            !almacen.prestado(TipoGafete::Contratista, numero(25)),
            "el gafete quedó libre"
        );
        assert_eq!(almacen.ultimo_movimiento(), Some(instante(SALIDA)));
    }

    #[tokio::test]
    async fn la_salida_por_gafete_encuentra_el_ingreso_abierto() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, Some(12)))
            .await
            .unwrap();
        salida(&almacen).por_gafete(&sesion(), 12).await.unwrap();
        assert!(!almacen.ingresos()[0].esta_abierto(), "cerrado");
        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 12).await,
            Err(ErrorCaso::NoEncontrado),
            "el 12 ya no está prestado a nadie"
        );
        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 0).await,
            Err(ErrorCaso::NoEncontrado),
            "el 0 no existe"
        );
    }

    #[tokio::test]
    async fn no_se_sale_dos_veces() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let ingreso = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, None))
            .await
            .unwrap()
            .ingreso;
        salida(&almacen).ejecutar(&sesion(), ingreso).await.unwrap();
        assert_eq!(
            salida(&almacen).ejecutar(&sesion(), ingreso).await,
            Err(ErrorCaso::Negocio(ErrorSalida::YaSalio))
        );
    }

    #[tokio::test]
    async fn la_salida_de_un_ingreso_que_no_existe_es_no_encontrado() {
        let almacen = preparado().await;
        let fantasma = limen_dominio::ingreso_contratista::IngresoId::desde_uuid(Uuid::nil());
        assert_eq!(
            salida(&almacen).ejecutar(&sesion(), fantasma).await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn con_el_reloj_atrasado_no_sale() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let ingreso = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, None))
            .await
            .unwrap()
            .ingreso;
        let reloj_atrasado = RegistrarSalida::new(
            almacen.clone(),
            reloj_a("2026-10-09T13:00:00Z"),
            almacen.ids(),
        );
        assert_eq!(
            reloj_atrasado.ejecutar(&sesion(), ingreso).await,
            Err(ErrorCaso::Negocio(ErrorSalida::Reloj(RelojAtrasado)))
        );
        assert!(almacen.ingresos()[0].esta_abierto(), "sigue adentro");
    }

    #[tokio::test]
    async fn despues_de_salir_puede_volver_a_entrar_con_el_mismo_gafete() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let ingreso = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, Some(8)))
            .await
            .unwrap()
            .ingreso;
        salida(&almacen).ejecutar(&sesion(), ingreso).await.unwrap();
        let de_nuevo = RegistrarEntrada::new(almacen.clone(), reloj_a(SALIDA), almacen.ids());
        de_nuevo
            .ejecutar(&sesion(), &a_pie(id, Some(8)))
            .await
            .unwrap();
        assert_eq!(almacen.ingresos().len(), 2, "dos ingresos en el historial");
    }

    // --- Gafetes ---

    #[tokio::test]
    async fn registrar_un_rango_crea_y_audita_cada_gafete() {
        let almacen = AlmacenMemoria::new();
        let creados = gafetes(&almacen)
            .ejecutar(&sesion(), TipoGafete::Visita, 10, 12)
            .await
            .unwrap();
        assert_eq!(creados, [numero(10), numero(11), numero(12)]);
        assert_eq!(almacen.gafetes().len(), 3);
        let auditoria = almacen.auditoria();
        assert_eq!(auditoria.len(), 3, "un alta por gafete");
        assert_eq!(
            auditoria[0].registro,
            RegistroAuditado::Gafete(TipoGafete::Visita, numero(10))
        );
    }

    #[tokio::test]
    async fn si_un_numero_del_rango_ya_existe_no_se_crea_ninguno() {
        let almacen = preparado().await;
        let antes = almacen.gafetes().len();
        assert_eq!(
            gafetes(&almacen)
                .ejecutar(&sesion(), TipoGafete::Contratista, 28, 35)
                .await,
            Err(ErrorCaso::Negocio(ErrorGafete::Repetido))
        );
        assert_eq!(almacen.gafetes().len(), antes, "todo o nada");
    }

    #[tokio::test]
    async fn el_mismo_numero_puede_existir_en_otro_tipo() {
        let almacen = preparado().await;
        assert!(
            gafetes(&almacen)
                .ejecutar(&sesion(), TipoGafete::Proveedor, 1, 5)
                .await
                .is_ok(),
            "el 1 de proveedor no es el 1 de contratista"
        );
    }

    #[tokio::test]
    async fn un_rango_invalido_se_rechaza() {
        let almacen = AlmacenMemoria::new();
        assert_eq!(
            gafetes(&almacen)
                .ejecutar(&sesion(), TipoGafete::Contratista, 9, 3)
                .await,
            Err(ErrorCaso::Negocio(ErrorGafete::RangoInvalido))
        );
    }

    #[tokio::test]
    async fn el_choque_al_crear_se_ve_como_repetido() {
        let almacen = AlmacenMemoria::new();
        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::NumeroGafete));
        assert_eq!(
            gafetes(&almacen)
                .ejecutar(&sesion(), TipoGafete::Contratista, 1, 1)
                .await,
            Err(ErrorCaso::Negocio(ErrorGafete::Repetido))
        );
    }

    #[tokio::test]
    async fn perdido_resuelto_y_auditado() {
        let almacen = preparado().await;
        // Un gafete de contratista lo pierde un contratista (F5).
        let portador = Portador::Contratista(praind(&almacen, "111111111").await);
        let perdido = cambiar(&almacen)
            .ejecutar(
                &sesion(),
                TipoGafete::Contratista,
                4,
                CambioGafete::MarcarPerdido(portador),
            )
            .await
            .unwrap();
        assert_eq!(perdido[0].campo, "estado");
        cambiar(&almacen)
            .ejecutar(
                &sesion(),
                TipoGafete::Contratista,
                4,
                CambioGafete::Resolver(Resolucion::Aparecido),
            )
            .await
            .unwrap();
        let gafete = almacen
            .gafetes()
            .into_iter()
            .find(|g| g.numero() == numero(4))
            .unwrap();
        assert_eq!(gafete.estado(), EstadoGafete::Disponible, "volvió");
        let ediciones = almacen
            .auditoria()
            .into_iter()
            .filter(|e| e.accion == AccionAuditada::Edicion)
            .count();
        assert_eq!(ediciones, 2, "perdido y resuelto quedaron auditados");
    }

    #[tokio::test]
    async fn no_se_da_de_baja_un_gafete_prestado() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, Some(6)))
            .await
            .unwrap();
        assert_eq!(
            cambiar(&almacen)
                .ejecutar(
                    &sesion(),
                    TipoGafete::Contratista,
                    6,
                    CambioGafete::DarDeBaja
                )
                .await,
            Err(ErrorCaso::Negocio(ErrorGafete::EnUso))
        );
        assert!(
            cambiar(&almacen)
                .ejecutar(
                    &sesion(),
                    TipoGafete::Contratista,
                    9,
                    CambioGafete::DarDeBaja
                )
                .await
                .is_ok(),
            "uno libre sí"
        );
    }

    #[tokio::test]
    async fn cambiar_un_gafete_que_no_existe_es_no_encontrado() {
        let almacen = AlmacenMemoria::new();
        assert_eq!(
            cambiar(&almacen)
                .ejecutar(
                    &sesion(),
                    TipoGafete::Contratista,
                    1,
                    CambioGafete::DarDeBaja
                )
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
        assert_eq!(
            cambiar(&almacen)
                .ejecutar(
                    &sesion(),
                    TipoGafete::Contratista,
                    0,
                    CambioGafete::DarDeBaja
                )
                .await,
            Err(ErrorCaso::Negocio(ErrorGafete::NumeroInvalido))
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
    async fn la_entrada_y_la_salida_dejan_cada_una_su_hecho() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let ingreso = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, Some(25)))
            .await
            .unwrap()
            .ingreso;
        assert_eq!(almacen.hechos().len(), 1, "la entrada deja su hecho");
        let hecho = &almacen.hechos()[0];
        assert!(
            matches!(hecho.suceso(), Suceso::EntradaContratista(guardado)
                if guardado == &almacen.ingresos()[0]),
            "el hecho guarda el ingreso tal como se abrió: {hecho:?}"
        );
        assert_eq!(hecho.marca().en, instante(ENTRADA));

        salida(&almacen).ejecutar(&sesion(), ingreso).await.unwrap();
        entrada_y_salida(&almacen, Via::Contratista, ingreso.uuid());
        assert_eq!(almacen.hechos()[1].marca().en, instante(SALIDA));
    }

    #[tokio::test]
    async fn lo_rechazado_no_deja_hecho() {
        let almacen = preparado().await;
        let sin_acceso = con_contratista(
            &almacen,
            "222222222",
            TipoIngreso::Praind,
            "2027-01-01",
            false,
        )
        .await;
        let resultado = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(sin_acceso, None))
            .await;
        assert!(resultado.is_err(), "acceso denegado");
        assert_eq!(
            almacen.hechos(),
            Vec::new(),
            "una entrada rechazada no deja hecho"
        );

        let id = praind(&almacen, "111111111").await;
        let ingreso = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, None))
            .await
            .unwrap()
            .ingreso;
        salida(&almacen).ejecutar(&sesion(), ingreso).await.unwrap();
        assert!(
            salida(&almacen).ejecutar(&sesion(), ingreso).await.is_err(),
            "no sale dos veces"
        );
        assert_eq!(
            almacen.hechos().len(),
            2,
            "la salida rechazada no deja hecho"
        );
    }

    #[tokio::test]
    async fn a_un_praind_hay_que_indicarle_gafete_o_sin_gafete() {
        let almacen = preparado().await;
        let id = praind(&almacen, "111111111").await;
        let nada = ComandoEntrada {
            gafete: None,
            ..a_pie(id, None)
        };
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &nada).await,
            Err(ErrorCaso::Negocio(ErrorIngreso::GafeteRequerido)),
        );
        assert_eq!(almacen.ingresos().len(), 0, "no se registra nada");

        let sin_gafete = entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(id, None))
            .await
            .unwrap();
        assert_eq!(
            almacen.ingresos()[0].entrega_gafete(),
            EntregaGafete::SinGafete,
            "S/G queda guardado"
        );
        assert_eq!(almacen.ingresos()[0].id(), sin_gafete.ingreso);
    }

    // --- Preparar el ingreso: buscador y ficha ---

    fn preparar(almacen: &AlmacenMemoria) -> PrepararIngreso<AlmacenMemoria, RelojFijo> {
        PrepararIngreso::new(almacen.clone(), reloj_a(ENTRADA))
    }

    async fn llamado(
        almacen: &AlmacenMemoria,
        cedula: &str,
        nombre: &str,
        tiene_acceso: bool,
    ) -> ContratistaId {
        let comando = ComandoContratista {
            cedula: cedula.into(),
            nombre: nombre.into(),
            empresa: EmpresaId::desde_uuid(Uuid::from_u128(500)),
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: "2027-01-01".parse().unwrap(),
            tiene_acceso,
        };
        RegistrarContratista::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
            .ejecutar(&sesion(), &comando)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn el_buscador_encuentra_por_cedula_o_por_nombre_con_la_decision_tomada() {
        let almacen = preparado().await;
        let ana = llamado(&almacen, "111111111", "ana solano", true).await;
        llamado(&almacen, "222222222", "beto mora", true).await;

        for texto in ["1111", "solano", "ANA", "sol ana"] {
            let encontrados = preparar(&almacen).buscar(texto, 10).await.unwrap();
            let ids: Vec<ContratistaId> = encontrados.iter().map(|c| c.contratista.id()).collect();
            assert_eq!(ids, vec![ana], "buscar {texto:?}");
        }
        let candidato = preparar(&almacen).ficha(ana).await.unwrap();
        assert_eq!(candidato.decision, Ok(ResultadoAcceso::Permitido));
        assert_eq!(
            candidato.empresa.map(|e| e.to_string()),
            Some("ACME".to_owned())
        );
        assert_eq!(candidato.gafetes_perdidos, Vec::new());
        assert!(
            preparar(&almacen)
                .buscar("  ", 10)
                .await
                .unwrap()
                .is_empty(),
            "un texto vacío no trae nada"
        );
    }

    #[tokio::test]
    async fn la_ficha_decide_lo_mismo_que_el_registro() {
        let almacen = preparado().await;
        let sin_acceso = llamado(&almacen, "111111111", "ana solano", false).await;
        let adentro = llamado(&almacen, "222222222", "beto mora", true).await;
        entrada(&almacen)
            .ejecutar(&sesion(), &a_pie(adentro, None))
            .await
            .unwrap();

        let casos = [
            (
                sin_acceso,
                ErrorIngreso::AccesoDenegado(MotivoDenegacion::SinAcceso),
            ),
            (
                adentro,
                ErrorIngreso::YaEstaAdentro(YaEstaAdentro(Via::Contratista)),
            ),
        ];
        for (id, motivo) in casos {
            let ficha = preparar(&almacen).ficha(id).await.unwrap();
            assert_eq!(ficha.decision, Err(motivo), "la ficha");
            assert_eq!(
                entrada(&almacen)
                    .ejecutar(&sesion(), &a_pie(id, None))
                    .await,
                Err(ErrorCaso::Negocio(motivo)),
                "el registro dice lo mismo"
            );
        }
    }

    #[tokio::test]
    async fn la_ficha_informa_los_gafetes_perdidos_a_su_nombre() {
        let almacen = preparado().await;
        let ana = llamado(&almacen, "111111111", "ana solano", true).await;
        let beto = llamado(&almacen, "222222222", "beto mora", true).await;
        for (numero, portador) in [(4, ana), (9, beto)] {
            cambiar(&almacen)
                .ejecutar(
                    &sesion(),
                    TipoGafete::Contratista,
                    numero,
                    CambioGafete::MarcarPerdido(Portador::Contratista(portador)),
                )
                .await
                .unwrap();
        }
        let ficha = preparar(&almacen).ficha(ana).await.unwrap();
        assert_eq!(ficha.gafetes_perdidos, vec![numero(4)], "sólo el suyo");
        assert_eq!(
            ficha.decision,
            Ok(ResultadoAcceso::Permitido),
            "no impide la entrada"
        );
        assert_eq!(
            preparar(&almacen)
                .ficha(ContratistaId::desde_uuid(Uuid::from_u128(77)))
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }
}
