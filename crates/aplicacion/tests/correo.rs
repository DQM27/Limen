//! Pruebas de los casos de uso del ingreso por correo contra los dobles en
//! memoria.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use limen_aplicacion::casos_de_uso::contratistas::{ComandoContratista, RegistrarContratista};
    use limen_aplicacion::casos_de_uso::correo::{
        ComandoEntradaCorreo, RegistrarEntradaCorreo, RegistrarSalidaCorreo,
    };
    use limen_aplicacion::casos_de_uso::gafetes::RegistrarGafetes;
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::{ErrorPersistencia, Restriccion};
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::cedula::Cedula;
    use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
    use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, TipoGafete};
    use limen_dominio::ingreso_correo::{ErrorIngresoCorreo, ErrorMotivo};
    use limen_dominio::medio::TipoMedio;
    use limen_dominio::movimiento::ErrorSalida;
    use limen_dominio::presencia::{Via, YaEstaAdentro};
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_dominio::visitante::{ErrorVisitante, PersonaVetada};
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales, RelojFijo};
    use uuid::Uuid;

    const ENTRADA: &str = "2026-10-09T15:00:00Z";
    const SALIDA: &str = "2026-10-09T16:30:00Z";
    const CEDULA: &str = "1-1111-1111";

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

    fn cedula() -> Cedula {
        Cedula::normalizar(CEDULA).unwrap()
    }

    fn entrada(
        almacen: &AlmacenMemoria,
    ) -> RegistrarEntradaCorreo<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        RegistrarEntradaCorreo::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
    }

    fn salida(almacen: &AlmacenMemoria) -> RegistrarSalidaCorreo<AlmacenMemoria, RelojFijo> {
        RegistrarSalidaCorreo::new(almacen.clone(), reloj_a(SALIDA))
    }

    /// Almacén con los gafetes de visita 1 a 5.
    async fn preparado() -> AlmacenMemoria {
        let almacen = AlmacenMemoria::new();
        RegistrarGafetes::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
            .ejecutar(&sesion(), TipoGafete::Visita, 1, 5)
            .await
            .unwrap();
        almacen
    }

    fn comando() -> ComandoEntradaCorreo {
        ComandoEntradaCorreo {
            cedula: CEDULA.into(),
            nombre: "josé peña".into(),
            motivo: "  Entrevista   con Recursos Humanos ".into(),
            medio: TipoMedio::APie,
            placa: None,
            gafete: 2,
        }
    }

    fn negocio(error: ErrorIngresoCorreo) -> ErrorCaso<ErrorIngresoCorreo> {
        ErrorCaso::Negocio(error)
    }

    #[tokio::test]
    async fn la_entrada_guarda_el_ingreso_la_presencia_el_prestamo_y_el_reloj() {
        let almacen = preparado().await;
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();

        let ingresos = almacen.ingresos_correo();
        assert_eq!(ingresos.len(), 1, "un ingreso");
        assert_eq!(ingresos[0].id(), id);
        assert_eq!(ingresos[0].cedula(), &cedula(), "cédula normalizada");
        assert_eq!(ingresos[0].visitante().nombre().as_str(), "JOSE PEÑA");
        assert_eq!(
            ingresos[0].motivo().as_str(),
            "Entrevista con Recursos Humanos",
            "el motivo se limpia pero no se pasa a mayúsculas"
        );
        assert_eq!(almacen.via_adentro(&cedula()), Some(Via::Correo));
        assert!(
            almacen.prestado(TipoGafete::Visita, NumeroGafete::nuevo(2).unwrap()),
            "gafete de visita prestado"
        );
        assert_eq!(almacen.ultimo_movimiento(), Some(instante(ENTRADA)));
    }

    #[tokio::test]
    async fn la_persona_y_el_motivo_se_validan_antes_de_leer_la_base() {
        let almacen = preparado().await;
        almacen.fallar_lecturas(ErrorPersistencia::Tecnica("no debería leer".into()));
        let mut sin_cedula = comando();
        sin_cedula.cedula = String::new();
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &sin_cedula).await,
            Err(negocio(ErrorIngresoCorreo::Visitante(
                ErrorVisitante::CedulaVacia
            )))
        );
        let mut sin_motivo = comando();
        sin_motivo.motivo = "  ".into();
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &sin_motivo).await,
            Err(negocio(ErrorIngresoCorreo::Motivo(ErrorMotivo::Vacio)))
        );
    }

    #[tokio::test]
    async fn el_gafete_debe_ser_de_visita() {
        let almacen = preparado().await;
        RegistrarGafetes::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
            .ejecutar(&sesion(), TipoGafete::Proveedor, 40, 40)
            .await
            .unwrap();
        let mut de_proveedor = comando();
        de_proveedor.gafete = 40;
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &de_proveedor).await,
            Err(negocio(ErrorIngresoCorreo::Gafete(
                ErrorPrestamoGafete::NoRegistrado
            ))),
            "el 40 sólo existe como gafete de proveedor"
        );
        let mut cero = comando();
        cero.gafete = 0;
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &cero).await,
            Err(negocio(ErrorIngresoCorreo::Gafete(
                ErrorPrestamoGafete::NoRegistrado
            )))
        );
    }

    #[tokio::test]
    async fn en_vehiculo_la_placa_es_obligatoria() {
        let almacen = preparado().await;
        let mut en_carro = comando();
        en_carro.medio = TipoMedio::Vehiculo;
        assert!(
            matches!(
                entrada(&almacen).ejecutar(&sesion(), &en_carro).await,
                Err(ErrorCaso::Negocio(ErrorIngresoCorreo::Medio(_)))
            ),
            "sin placa no entra en vehículo"
        );
    }

    #[tokio::test]
    async fn no_entra_si_ya_esta_adentro_por_cualquier_via() {
        for via in [Via::Contratista, Via::Proveedor, Via::Correo] {
            let almacen = preparado().await;
            almacen.marcar_adentro(&cedula(), via);
            assert_eq!(
                entrada(&almacen).ejecutar(&sesion(), &comando()).await,
                Err(negocio(ErrorIngresoCorreo::YaEstaAdentro(YaEstaAdentro(
                    via
                )))),
                "adentro como {via}"
            );
        }
    }

    #[tokio::test]
    async fn la_cedula_vetada_como_contratista_no_entra_por_correo() {
        let almacen = preparado().await;
        almacen.sembrar_empresa(Empresa::restaurar(
            EmpresaId::desde_uuid(Uuid::from_u128(500)),
            NombreEmpresa::nuevo("ACME").unwrap(),
        ));
        let contratista =
            RegistrarContratista::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
                .ejecutar(
                    &sesion(),
                    &ComandoContratista {
                        cedula: CEDULA.into(),
                        nombre: "josé peña".into(),
                        empresa: EmpresaId::desde_uuid(Uuid::from_u128(500)),
                        tipo_ingreso: TipoIngreso::Praind,
                        fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
                        tiene_acceso: false,
                    },
                )
                .await
                .unwrap();
        assert!(!almacen.contratistas().is_empty(), "{contratista}");
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &comando()).await,
            Err(negocio(ErrorIngresoCorreo::AccesoDenegado(PersonaVetada)))
        );
        assert!(almacen.ingresos_correo().is_empty(), "no se registró nada");
    }

    #[tokio::test]
    async fn no_presta_un_gafete_ya_prestado_ni_con_el_reloj_atrasado() {
        let almacen = preparado().await;
        entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        let mut otra = comando();
        otra.cedula = "2-2222-2222".into();
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &otra).await,
            Err(negocio(ErrorIngresoCorreo::Gafete(
                ErrorPrestamoGafete::Prestado
            )))
        );

        let almacen = preparado().await;
        almacen.fijar_ultimo_movimiento(instante("2026-10-09T18:00:00Z"));
        assert!(
            matches!(
                entrada(&almacen).ejecutar(&sesion(), &comando()).await,
                Err(ErrorCaso::Negocio(ErrorIngresoCorreo::Reloj(_)))
            ),
            "reloj atrasado"
        );
    }

    #[tokio::test]
    async fn los_choques_con_otro_equipo_llegan_como_errores_de_negocio() {
        let almacen = preparado().await;
        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(
            Restriccion::PresenciaPersona,
        ));
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &comando()).await,
            Err(negocio(ErrorIngresoCorreo::YaEstaAdentro(YaEstaAdentro(
                Via::Correo
            ))))
        );
        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::GafetePrestado));
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &comando()).await,
            Err(negocio(ErrorIngresoCorreo::Gafete(
                ErrorPrestamoGafete::Prestado
            )))
        );
        assert!(almacen.ingresos_correo().is_empty(), "nada se aplicó");
    }

    #[tokio::test]
    async fn la_salida_por_gafete_libera_la_presencia_y_el_gafete() {
        let almacen = preparado().await;
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        salida(&almacen).por_gafete(&sesion(), 2).await.unwrap();

        assert_eq!(
            almacen.ingresos_correo()[0].salida().unwrap().en,
            instante(SALIDA)
        );
        assert_eq!(almacen.via_adentro(&cedula()), None, "ya no está adentro");
        assert!(
            !almacen.prestado(TipoGafete::Visita, NumeroGafete::nuevo(2).unwrap()),
            "el gafete quedó libre"
        );
        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 2).await,
            Err(ErrorCaso::NoEncontrado),
            "ya no hay un ingreso abierto con ese gafete"
        );
        assert_eq!(
            salida(&almacen).ejecutar(&sesion(), id).await,
            Err(ErrorCaso::Negocio(ErrorSalida::YaSalio))
        );
    }

    #[tokio::test]
    async fn la_salida_por_ingreso_cierra_y_el_gafete_se_puede_reusar() {
        let almacen = preparado().await;
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        salida(&almacen).ejecutar(&sesion(), id).await.unwrap();
        assert!(!almacen.ingresos_correo()[0].esta_abierto(), "cerrado");
        RegistrarEntradaCorreo::new(almacen.clone(), reloj_a(SALIDA), almacen.ids())
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn salidas_que_no_existen_o_con_gafete_cero() {
        let almacen = preparado().await;
        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 2).await,
            Err(ErrorCaso::NoEncontrado),
            "nadie tiene el 2"
        );
        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 0).await,
            Err(ErrorCaso::NoEncontrado)
        );
    }
}
