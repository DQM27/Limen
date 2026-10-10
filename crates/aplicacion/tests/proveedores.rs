//! Pruebas de los casos de uso de proveedores contra los dobles en memoria.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use limen_aplicacion::casos_de_uso::gafetes::RegistrarGafetes;
    use limen_aplicacion::casos_de_uso::proveedores::{
        ComandoEntradaProveedor, RegistrarEmpresaProveedora, RegistrarEntradaProveedor,
        RegistrarSalidaProveedor, RenombrarEmpresaProveedora,
    };
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::{
        AccionAuditada, ErrorPersistencia, RegistroAuditado, Restriccion,
    };
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::cedula::Cedula;
    use limen_dominio::contratista::{Contratista, ContratistaGuardado, ContratistaId};
    use limen_dominio::empresa::{EmpresaId, ErrorEmpresa, NombreEmpresa};
    use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
    use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, TipoGafete};
    use limen_dominio::ingreso_proveedor::ErrorIngresoProveedor;
    use limen_dominio::medio::{Medio, Placa, TipoMedio};
    use limen_dominio::movimiento::ErrorSalida;
    use limen_dominio::nombre::NombrePersona;
    use limen_dominio::presencia::{Via, YaEstaAdentro};
    use limen_dominio::reloj::RelojAtrasado;
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_dominio::visitante::{ErrorVisitante, PersonaVetada};
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales, RelojFijo};
    use uuid::Uuid;

    // --- Preparación ---

    const ENTRADA: &str = "2026-10-09T14:00:00Z";
    const SALIDA: &str = "2026-10-09T16:00:00Z";
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

    fn numero(n: u32) -> NumeroGafete {
        NumeroGafete::nuevo(n).unwrap()
    }

    fn id_empresa() -> EmpresaProveedoraId {
        EmpresaProveedoraId::desde_uuid(Uuid::from_u128(700))
    }

    type Entrada = RegistrarEntradaProveedor<AlmacenMemoria, RelojFijo, IdsSecuenciales>;

    fn entrada(almacen: &AlmacenMemoria) -> Entrada {
        RegistrarEntradaProveedor::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
    }

    fn salida(
        almacen: &AlmacenMemoria,
    ) -> RegistrarSalidaProveedor<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        RegistrarSalidaProveedor::new(almacen.clone(), reloj_a(SALIDA), almacen.ids())
    }

    /// Almacén con la empresa proveedora GAS ZETA y los gafetes de
    /// proveedor 1 a 10.
    async fn preparado() -> AlmacenMemoria {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_empresa_proveedora(EmpresaProveedora::restaurar(
            id_empresa(),
            NombreEmpresa::nuevo("GAS ZETA").unwrap(),
        ));
        RegistrarGafetes::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
            .ejecutar(&sesion(), TipoGafete::Proveedor, 1, 10)
            .await
            .unwrap();
        almacen
    }

    fn comando() -> ComandoEntradaProveedor {
        ComandoEntradaProveedor {
            cedula: CEDULA.into(),
            nombre: "josé peña".into(),
            empresa: id_empresa(),
            medio: TipoMedio::Vehiculo,
            placa: Some("abc-123".into()),
            gafete: 4,
        }
    }

    /// Un contratista con la misma cédula, con o sin acceso.
    fn contratista(tiene_acceso: bool) -> Contratista {
        Contratista::restaurar(ContratistaGuardado {
            id: ContratistaId::desde_uuid(Uuid::from_u128(800)),
            cedula: cedula(),
            nombre: NombrePersona::nuevo("JOSE PEÑA").unwrap(),
            empresa: EmpresaId::desde_uuid(Uuid::from_u128(801)),
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            tiene_acceso,
        })
    }

    fn negocio(error: ErrorIngresoProveedor) -> ErrorCaso<ErrorIngresoProveedor> {
        ErrorCaso::Negocio(error)
    }

    // --- Empresas proveedoras ---

    #[tokio::test]
    async fn registra_una_empresa_proveedora_y_audita_el_alta() {
        let almacen = AlmacenMemoria::new();
        let id = RegistrarEmpresaProveedora::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
            .ejecutar(&sesion(), " gas  zeta ")
            .await
            .unwrap();

        let empresas = almacen.empresas_proveedoras();
        assert_eq!(empresas.len(), 1, "quedó guardada");
        assert_eq!(empresas[0].nombre().as_str(), "GAS ZETA");
        assert!(
            almacen.empresas().is_empty(),
            "no toca el catálogo de empresas de contratistas"
        );
        let auditoria = almacen.auditoria();
        assert_eq!(auditoria.len(), 1, "una entrada de auditoría");
        assert_eq!(
            auditoria[0].registro,
            RegistroAuditado::EmpresaProveedora(id)
        );
        assert_eq!(auditoria[0].accion, AccionAuditada::Alta);
    }

    #[tokio::test]
    async fn el_nombre_de_empresa_proveedora_no_se_repite() {
        let almacen = preparado().await;
        let registrar =
            RegistrarEmpresaProveedora::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids());
        assert_eq!(
            registrar.ejecutar(&sesion(), "gas zeta").await,
            Err(ErrorCaso::Negocio(ErrorEmpresa::NombreRepetido))
        );

        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(
            Restriccion::NombreEmpresaProveedora,
        ));
        assert_eq!(
            registrar.ejecutar(&sesion(), "OTRA").await,
            Err(ErrorCaso::Negocio(ErrorEmpresa::NombreRepetido)),
            "si otro equipo lo guardó primero, es el mismo error"
        );
    }

    #[tokio::test]
    async fn renombrar_una_empresa_proveedora_audita_el_cambio() {
        let almacen = preparado().await;
        let renombrar =
            RenombrarEmpresaProveedora::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids());
        let cambios = renombrar
            .ejecutar(&sesion(), id_empresa(), "gas zeta s.a.")
            .await
            .unwrap();
        assert_eq!(cambios.len(), 1, "cambió el nombre");
        assert_eq!(
            almacen.empresas_proveedoras()[0].nombre().as_str(),
            "GAS ZETA S.A."
        );
        let ultima = almacen.auditoria().pop().unwrap();
        assert_eq!(ultima.accion, AccionAuditada::Edicion);
        assert_eq!(
            ultima.registro,
            RegistroAuditado::EmpresaProveedora(id_empresa())
        );

        let antes = almacen.confirmaciones();
        assert_eq!(
            renombrar
                .ejecutar(&sesion(), id_empresa(), "GAS ZETA S.A.")
                .await,
            Ok(Vec::new()),
            "el mismo nombre no cambia nada"
        );
        assert_eq!(almacen.confirmaciones(), antes, "y no escribe");
        assert_eq!(
            renombrar
                .ejecutar(
                    &sesion(),
                    EmpresaProveedoraId::desde_uuid(Uuid::from_u128(1)),
                    "X"
                )
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    // --- Entrada ---

    #[tokio::test]
    async fn la_entrada_guarda_el_ingreso_la_presencia_el_prestamo_y_el_reloj() {
        let almacen = preparado().await;
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();

        let ingresos = almacen.ingresos_proveedor();
        assert_eq!(ingresos.len(), 1, "un ingreso");
        let ingreso = &ingresos[0];
        assert_eq!(ingreso.id(), id);
        assert_eq!(ingreso.cedula(), &cedula(), "cédula normalizada");
        assert_eq!(ingreso.visitante().nombre().as_str(), "JOSE PEÑA");
        assert_eq!(ingreso.empresa(), id_empresa());
        assert_eq!(
            ingreso.medio(),
            &Medio::Vehiculo(Placa::nueva("ABC-123").unwrap())
        );
        assert_eq!(ingreso.gafete(), numero(4));
        assert_eq!(ingreso.entrada().operador, sesion().operador());
        assert_eq!(almacen.via_adentro(&cedula()), Some(Via::Proveedor));
        assert!(
            almacen.prestado(TipoGafete::Proveedor, numero(4)),
            "prestado"
        );
        assert_eq!(almacen.ultimo_movimiento(), Some(instante(ENTRADA)));
    }

    #[tokio::test]
    async fn la_persona_se_valida_antes_de_leer_la_base() {
        let almacen = preparado().await;
        almacen.fallar_lecturas(ErrorPersistencia::Tecnica("no debería leer".into()));
        let mut sin_cedula = comando();
        sin_cedula.cedula = " ".into();
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &sin_cedula).await,
            Err(negocio(ErrorIngresoProveedor::Visitante(
                ErrorVisitante::CedulaVacia
            )))
        );
        let mut nombre_malo = comando();
        nombre_malo.nombre = "ANA 2".into();
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &nombre_malo).await,
            Err(negocio(ErrorIngresoProveedor::Visitante(
                ErrorVisitante::NombreInvalido
            )))
        );
    }

    #[tokio::test]
    async fn el_gafete_cero_no_existe() {
        let almacen = preparado().await;
        let mut cero = comando();
        cero.gafete = 0;
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &cero).await,
            Err(negocio(ErrorIngresoProveedor::Gafete(
                ErrorPrestamoGafete::NoRegistrado
            )))
        );
    }

    #[tokio::test]
    async fn el_gafete_debe_ser_de_proveedor() {
        let almacen = preparado().await;
        RegistrarGafetes::new(almacen.clone(), reloj_a(ENTRADA), almacen.ids())
            .ejecutar(&sesion(), TipoGafete::Contratista, 50, 50)
            .await
            .unwrap();
        let mut de_contratista = comando();
        de_contratista.gafete = 50;
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &de_contratista).await,
            Err(negocio(ErrorIngresoProveedor::Gafete(
                ErrorPrestamoGafete::NoRegistrado
            ))),
            "el 50 sólo existe como gafete de contratista"
        );
    }

    #[tokio::test]
    async fn la_empresa_proveedora_debe_existir() {
        let almacen = preparado().await;
        let mut otra = comando();
        otra.empresa = EmpresaProveedoraId::desde_uuid(Uuid::from_u128(1));
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &otra).await,
            Err(negocio(ErrorIngresoProveedor::EmpresaNoExiste))
        );
    }

    #[tokio::test]
    async fn no_entra_si_ya_esta_adentro_por_otra_via() {
        let almacen = preparado().await;
        almacen.marcar_adentro(&cedula(), Via::Contratista);
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &comando()).await,
            Err(negocio(ErrorIngresoProveedor::YaEstaAdentro(
                YaEstaAdentro(Via::Contratista)
            )))
        );
    }

    #[tokio::test]
    async fn la_cedula_vetada_como_contratista_no_entra_como_proveedor() {
        let almacen = preparado().await;
        almacen.sembrar_contratista(contratista(false));
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &comando()).await,
            Err(negocio(ErrorIngresoProveedor::AccesoDenegado(
                PersonaVetada
            )))
        );
        assert!(
            almacen.ingresos_proveedor().is_empty(),
            "no se registró nada"
        );
    }

    #[tokio::test]
    async fn un_contratista_con_acceso_puede_entrar_como_proveedor() {
        let almacen = preparado().await;
        almacen.sembrar_contratista(contratista(true));
        assert!(
            entrada(&almacen)
                .ejecutar(&sesion(), &comando())
                .await
                .is_ok(),
            "sólo el acceso denegado veta"
        );
    }

    #[tokio::test]
    async fn no_presta_un_gafete_que_ya_esta_prestado() {
        let almacen = preparado().await;
        entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        let mut otra_persona = comando();
        otra_persona.cedula = "2-2222-2222".into();
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &otra_persona).await,
            Err(negocio(ErrorIngresoProveedor::Gafete(
                ErrorPrestamoGafete::Prestado
            )))
        );
    }

    #[tokio::test]
    async fn no_registra_con_el_reloj_atrasado() {
        let almacen = preparado().await;
        almacen.fijar_ultimo_movimiento(instante("2026-10-09T15:00:00Z"));
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &comando()).await,
            Err(negocio(ErrorIngresoProveedor::Reloj(RelojAtrasado)))
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
            Err(negocio(ErrorIngresoProveedor::YaEstaAdentro(
                YaEstaAdentro(Via::Proveedor)
            )))
        );
        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::GafetePrestado));
        assert_eq!(
            entrada(&almacen).ejecutar(&sesion(), &comando()).await,
            Err(negocio(ErrorIngresoProveedor::Gafete(
                ErrorPrestamoGafete::Prestado
            )))
        );
        assert!(almacen.ingresos_proveedor().is_empty(), "nada se aplicó");
    }

    // --- Salida ---

    #[tokio::test]
    async fn la_salida_por_gafete_libera_la_presencia_y_el_gafete() {
        let almacen = preparado().await;
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        salida(&almacen).por_gafete(&sesion(), 4).await.unwrap();

        let ingreso = &almacen.ingresos_proveedor()[0];
        assert_eq!(ingreso.id(), id);
        assert_eq!(ingreso.salida().unwrap().en, instante(SALIDA));
        assert_eq!(almacen.via_adentro(&cedula()), None, "ya no está adentro");
        assert!(
            !almacen.prestado(TipoGafete::Proveedor, numero(4)),
            "el gafete quedó libre"
        );
        assert_eq!(almacen.ultimo_movimiento(), Some(instante(SALIDA)));

        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 4).await,
            Err(ErrorCaso::NoEncontrado),
            "no queda un ingreso abierto con ese gafete"
        );
        assert_eq!(
            salida(&almacen).ejecutar(&sesion(), id).await,
            Err(ErrorCaso::Negocio(ErrorSalida::YaSalio))
        );
    }

    #[tokio::test]
    async fn la_salida_por_ingreso_tambien_cierra() {
        let almacen = preparado().await;
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        salida(&almacen).ejecutar(&sesion(), id).await.unwrap();
        assert!(
            !almacen.ingresos_proveedor()[0].esta_abierto(),
            "quedó cerrado"
        );
        // Ya afuera, puede volver a entrar con el mismo gafete.
        RegistrarEntradaProveedor::new(almacen.clone(), reloj_a(SALIDA), almacen.ids())
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn la_salida_por_gafete_no_confunde_tipos_ni_acepta_el_cero() {
        let almacen = preparado().await;
        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 4).await,
            Err(ErrorCaso::NoEncontrado),
            "nadie tiene el 4"
        );
        assert_eq!(
            salida(&almacen).por_gafete(&sesion(), 0).await,
            Err(ErrorCaso::NoEncontrado)
        );
        assert_eq!(
            salida(&almacen)
                .ejecutar(
                    &sesion(),
                    limen_dominio::ingreso_proveedor::IngresoProveedorId::desde_uuid(
                        Uuid::from_u128(1)
                    )
                )
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn la_salida_respeta_el_reloj() {
        let almacen = preparado().await;
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        assert_eq!(
            RegistrarSalidaProveedor::new(
                almacen.clone(),
                reloj_a("2026-10-09T13:00:00Z"),
                almacen.ids()
            )
            .ejecutar(&sesion(), id)
            .await,
            Err(ErrorCaso::Negocio(ErrorSalida::Reloj(RelojAtrasado))),
            "la hora es anterior al último movimiento"
        );
        assert_eq!(
            almacen.via_adentro(&cedula()),
            Some(Via::Proveedor),
            "sigue adentro"
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
        let id = entrada(&almacen)
            .ejecutar(&sesion(), &comando())
            .await
            .unwrap();
        salida(&almacen).ejecutar(&sesion(), id).await.unwrap();
        entrada_y_salida(&almacen, Via::Proveedor, id.uuid());
    }
}
