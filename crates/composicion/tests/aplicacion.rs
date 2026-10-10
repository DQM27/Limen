//! Pruebas de la aplicación completa, armada como en producción (`SurrealDB`,
//! hora de Costa Rica y UUID v7) y también con los dobles en memoria.

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use limen_aplicacion::casos_de_uso::contratistas::ComandoContratista;
    use limen_aplicacion::casos_de_uso::correo::ComandoEntradaCorreo;
    use limen_aplicacion::casos_de_uso::ingresos::{ComandoEntrada, GafeteElegido};
    use limen_aplicacion::casos_de_uso::proveedores::ComandoEntradaProveedor;
    use limen_aplicacion::puertos::{
        AccionAuditada, Consultas, FabricaUnidadDeTrabajo, GeneradorIds, IngresoAbierto,
        RegistroAuditado, Reloj,
    };
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_composicion::{Aplicacion, AplicacionLimen, Config};
    use limen_dominio::acceso::ResultadoAcceso;
    use limen_dominio::cedula::Cedula;
    use limen_dominio::contratista::{Contratista, ContratistaGuardado, ContratistaId};
    use limen_dominio::empresa_proveedora::EmpresaProveedoraId;
    use limen_dominio::gafete::TipoGafete;
    use limen_dominio::medio::TipoMedio;
    use limen_dominio::nombre::NombrePersona;
    use limen_dominio::personal_kof::PersonalKofId;
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_dominio::usuario::ErrorInicioSesion;
    use limen_infra_memoria::{AlmacenMemoria, ContrasenasFalsas, IdsSecuenciales, RelojFijo};
    use limen_infra_plataforma::{ContrasenasArgon2, IdsV7, RelojConfiable};
    use limen_infra_surreal::AlmacenSurreal;
    use uuid::Uuid;

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    /// La aplicación se comparte entre pantallas y comandos: tiene que poder
    /// viajar entre hilos.
    fn es_send_y_sync<T: Send + Sync>() {}

    #[test]
    fn la_aplicacion_se_puede_compartir_entre_hilos() {
        es_send_y_sync::<AplicacionLimen>();
        es_send_y_sync::<Aplicacion<AlmacenMemoria, RelojFijo, IdsSecuenciales, ContrasenasFalsas>>(
        );
    }

    /// Lo que se crea al preparar el día y se necesita después.
    struct Dia {
        gas: EmpresaProveedoraId,
        contratista: ContratistaId,
        michael: PersonalKofId,
    }

    /// Catálogos: empresas, gafetes, un contratista y una persona del KOF; y
    /// comprueba que el buscador los encuentra.
    async fn preparar_el_dia<F, R, G, C>(app: &Aplicacion<F, R, G, C>) -> Dia
    where
        F: FabricaUnidadDeTrabajo + Consultas + Clone,
        R: Reloj + Clone,
        G: GeneradorIds + Clone,
        C: Sync,
    {
        let sesion = sesion();
        let acme = app
            .empresas
            .registrar
            .ejecutar(&sesion, "acme s.a.")
            .await
            .unwrap();
        let gas = app
            .proveedores
            .registrar_empresa
            .ejecutar(&sesion, "gas zeta")
            .await
            .unwrap();
        for (tipo, desde, hasta) in [
            (TipoGafete::Contratista, 1, 10),
            (TipoGafete::Proveedor, 1, 5),
            (TipoGafete::Visita, 1, 5),
            (TipoGafete::ProvisionalKof, 1, 5),
        ] {
            app.gafetes
                .registrar
                .ejecutar(&sesion, tipo, desde, hasta)
                .await
                .unwrap();
        }
        let contratista = app
            .contratistas
            .registrar
            .ejecutar(
                &sesion,
                &ComandoContratista {
                    cedula: "1-1111-1111".into(),
                    nombre: "josé peña".into(),
                    empresa: acme,
                    tipo_ingreso: TipoIngreso::Praind,
                    fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2099, 1, 1).unwrap(),
                    tiene_acceso: true,
                },
            )
            .await
            .unwrap();
        let michael = app
            .kof
            .registrar
            .ejecutar(&sesion, "5040017", "michael araya")
            .await
            .unwrap();

        let encontrados = app
            .contratistas
            .buscar
            .ejecutar("jose pe", 10)
            .await
            .unwrap();
        assert_eq!(encontrados.len(), 1, "un contratista");
        assert_eq!(encontrados[0].id(), contratista, "el que se registró");
        assert_eq!(
            app.empresas.buscar.ejecutar("acm", 10).await.unwrap().len(),
            1,
            "la empresa de contratistas"
        );
        assert_eq!(
            app.proveedores
                .buscar_empresas
                .ejecutar("gas ze", 10)
                .await
                .unwrap()
                .len(),
            1,
            "la empresa proveedora"
        );
        assert_eq!(
            app.kof.buscar.ejecutar("5040", 10).await.unwrap().len(),
            1,
            "la persona del KOF"
        );
        Dia {
            gas,
            contratista,
            michael,
        }
    }

    /// Entran por las tres vías y se le presta un provisional al KOF.
    async fn entran_todos<F, R, G, C>(app: &Aplicacion<F, R, G, C>, dia: &Dia)
    where
        F: FabricaUnidadDeTrabajo + Consultas + Clone,
        R: Reloj + Clone,
        G: GeneradorIds + Clone,
        C: Sync,
    {
        let sesion = sesion();
        let entrada = app
            .ingresos
            .entrada
            .ejecutar(
                &sesion,
                &ComandoEntrada {
                    contratista: dia.contratista,
                    medio: TipoMedio::APie,
                    placa: None,
                    gafete: Some(GafeteElegido::Numero(3)),
                },
            )
            .await
            .unwrap();
        assert_eq!(entrada.acceso, ResultadoAcceso::Permitido, "PRAIND vigente");
        app.proveedores
            .entrada
            .ejecutar(
                &sesion,
                &ComandoEntradaProveedor {
                    cedula: "2-2222-2222".into(),
                    nombre: "beto solís".into(),
                    empresa: dia.gas,
                    medio: TipoMedio::Vehiculo,
                    placa: Some("abc-123".into()),
                    gafete: 2,
                },
            )
            .await
            .unwrap();
        app.correo
            .entrada
            .ejecutar(
                &sesion,
                &ComandoEntradaCorreo {
                    cedula: "3-3333-3333".into(),
                    nombre: "luis mora".into(),
                    motivo: "Entrevista con RH".into(),
                    medio: TipoMedio::APie,
                    placa: None,
                    gafete: 4,
                },
            )
            .await
            .unwrap();
        app.kof
            .entregar_gafete
            .ejecutar(&sesion, dia.michael, 1)
            .await
            .unwrap();

        let adentro = app.quienes_estan_adentro.ejecutar().await.unwrap();
        assert_eq!(
            adentro.len(),
            4,
            "contratista, proveedor, visita y personal KOF"
        );
        let vias: Vec<_> = adentro
            .iter()
            .map(|p| p.ingreso.via().to_string())
            .collect();
        for via in [
            "contratista",
            "proveedor",
            "ingreso por correo",
            "personal KOF",
        ] {
            assert!(
                vias.iter().any(|v| v == via),
                "{via} está adentro: {vias:?}"
            );
        }
        assert!(
            adentro
                .iter()
                .any(|p| p.nombre.as_str() == "JOSE PEÑA" && p.procedencia == "ACME S.A."),
            "el contratista con su empresa"
        );
        assert!(
            adentro
                .iter()
                .any(|p| matches!(p.ingreso, IngresoAbierto::Proveedor(_))
                    && p.procedencia == "GAS ZETA"),
            "el proveedor con su empresa"
        );
        assert!(
            adentro
                .iter()
                .any(|p| matches!(p.ingreso, IngresoAbierto::Kof(_))
                    && p.identidad.to_string() == "5040017"
                    && p.medio.is_none()),
            "el personal KOF por su código de empleado, sin medio"
        );
    }

    /// Las consultas de apoyo ven lo que pasó durante el día.
    async fn consultas_de_apoyo<F, R, G, C>(app: &Aplicacion<F, R, G, C>, dia: &Dia)
    where
        F: FabricaUnidadDeTrabajo + Consultas + Clone,
        R: Reloj + Clone,
        G: GeneradorIds + Clone,
        C: Sync,
    {
        let historial = app
            .historial
            .ejecutar(RegistroAuditado::Contratista(dia.contratista))
            .await
            .unwrap();
        assert_eq!(historial.len(), 1, "sólo el alta");
        assert_eq!(historial[0].accion, AccionAuditada::Alta, "fue el alta");

        let gafetes = app
            .gafetes
            .listar
            .ejecutar(TipoGafete::Contratista)
            .await
            .unwrap();
        assert_eq!(gafetes.len(), 10, "los diez gafetes de contratista");
        let prestados: Vec<u32> = gafetes
            .iter()
            .filter(|resumen| resumen.prestado)
            .map(|resumen| resumen.gafete.numero().valor())
            .collect();
        assert_eq!(
            prestados,
            [3],
            "el 3 está prestado al contratista de adentro"
        );

        assert!(
            app.contratistas
                .praind_por_vencer
                .ejecutar()
                .await
                .unwrap()
                .is_empty(),
            "su PRAIND vence en 2099"
        );
    }

    /// Salen, cada uno por su gafete.
    async fn salen_todos<F, R, G, C>(app: &Aplicacion<F, R, G, C>)
    where
        F: FabricaUnidadDeTrabajo + Consultas + Clone,
        R: Reloj + Clone,
        G: GeneradorIds + Clone,
        C: Sync,
    {
        let sesion = sesion();
        app.ingresos.salida.por_gafete(&sesion, 3).await.unwrap();
        app.proveedores.salida.por_gafete(&sesion, 2).await.unwrap();
        app.correo.salida.por_gafete(&sesion, 4).await.unwrap();
        app.kof
            .devolver_gafete
            .por_gafete(&sesion, 1)
            .await
            .unwrap();
        assert!(
            app.quienes_estan_adentro
                .ejecutar()
                .await
                .unwrap()
                .is_empty(),
            "no queda nadie adentro"
        );
    }

    /// Un día completo en la portería, con cada vía de ingreso.
    async fn un_dia_en_la_porteria<F, R, G, C>(app: &Aplicacion<F, R, G, C>)
    where
        F: FabricaUnidadDeTrabajo + Consultas + Clone,
        R: Reloj + Clone,
        G: GeneradorIds + Clone,
        C: Sync,
    {
        let dia = preparar_el_dia(app).await;
        entran_todos(app, &dia).await;
        consultas_de_apoyo(app, &dia).await;
        salen_todos(app).await;
    }

    #[tokio::test]
    async fn un_dia_completo_con_los_adaptadores_reales() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let app = AplicacionLimen::nueva(
            &almacen,
            &RelojConfiable::new(None),
            &IdsV7,
            &ContrasenasArgon2::new().unwrap(),
        );
        un_dia_en_la_porteria(&app).await;
    }

    #[tokio::test]
    async fn el_primer_usuario_entra_con_argon2_y_surreal() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let app = AplicacionLimen::nueva(
            &almacen,
            &RelojConfiable::new(None),
            &IdsV7,
            &ContrasenasArgon2::new().unwrap(),
        );
        let usuarios = &app.usuarios;
        assert!(
            !usuarios.hay_usuarios.ejecutar().await.unwrap(),
            "base recién creada"
        );
        let creado = usuarios
            .crear_primero
            .ejecutar("1-1111-1111", "ana mora", "portería segura")
            .await
            .unwrap();
        let iniciada = usuarios
            .iniciar_sesion
            .ejecutar("111111111", "portería segura")
            .await
            .unwrap();
        assert_eq!(iniciada, creado, "entra el mismo usuario");
        assert_eq!(
            usuarios
                .iniciar_sesion
                .ejecutar("111111111", "otra cosa")
                .await
                .unwrap_err()
                .para_interfaz()
                .codigo,
            ErrorInicioSesion::CredencialesInvalidas.codigo(),
            "la contraseña equivocada no entra"
        );
    }

    #[tokio::test]
    async fn un_dia_completo_con_los_dobles_en_memoria() {
        let almacen = AlmacenMemoria::new();
        let reloj = RelojFijo::new(
            "2026-10-09T14:00:00Z".parse().unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        );
        let app = Aplicacion::nueva(&almacen, &reloj, &almacen.ids(), &ContrasenasFalsas);
        un_dia_en_la_porteria(&app).await;
    }

    #[tokio::test]
    async fn abrir_crea_la_base_en_disco_y_los_datos_sobreviven() {
        let carpeta = std::env::temp_dir().join(format!("limen-composicion-{}", Uuid::now_v7()));
        let config = Config {
            ruta_base: carpeta.clone(),
        };
        {
            let app = AplicacionLimen::abrir(&config, &RelojConfiable::new(None))
                .await
                .unwrap();
            app.empresas
                .registrar
                .ejecutar(&sesion(), "acme")
                .await
                .unwrap();
        }
        // El motor suelta el archivo en segundo plano: se reintenta mientras
        // siga tomado.
        let mut app = None;
        for _ in 0..100 {
            match AplicacionLimen::abrir(&config, &RelojConfiable::new(None)).await {
                Ok(abierta) => {
                    app = Some(abierta);
                    break;
                }
                // "locked" en Linux; "os error 33" en Windows.
                Err(error)
                    if error.to_string().contains("locked")
                        || error.to_string().contains("os error 33") =>
                {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                Err(error) => panic!("no se pudo reabrir: {error}"),
            }
        }
        let app = app.expect("el motor liberó el archivo");
        assert_eq!(
            app.empresas
                .buscar
                .ejecutar("acme", 10)
                .await
                .unwrap()
                .len(),
            1,
            "la empresa sigue ahí después de reabrir"
        );
        drop(app);
        drop(std::fs::remove_dir_all(carpeta));
    }

    #[tokio::test]
    async fn abrir_en_una_ruta_imposible_es_un_error_de_arranque() {
        // Una carpeta que "cuelga" de un archivo normal no se puede crear en
        // ningún sistema operativo.
        let archivo = std::env::temp_dir().join(format!("limen-archivo-{}", Uuid::now_v7()));
        std::fs::write(&archivo, b"no soy una carpeta").unwrap();
        let config = Config {
            ruta_base: archivo.join("base"),
        };
        let error = AplicacionLimen::abrir(&config, &RelojConfiable::new(None))
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("No se pudo abrir la base de datos"),
            "{error}"
        );
        drop(std::fs::remove_file(archivo));
    }

    #[tokio::test]
    async fn el_praind_por_vencer_usa_los_30_dias_de_advertencia_del_reloj() {
        let almacen = AlmacenMemoria::new();
        let hoy = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        let reloj = RelojFijo::new("2026-10-09T14:00:00Z".parse().unwrap(), hoy);
        let app = Aplicacion::nueva(&almacen, &reloj, &almacen.ids(), &ContrasenasFalsas);
        let sesion = sesion();
        let acme = app
            .empresas
            .registrar
            .ejecutar(&sesion, "acme")
            .await
            .unwrap();
        // Alguien cuyo PRAIND venció después de registrarlo (hoy no se
        // puede registrar uno vencido: regla B6).
        almacen.sembrar_contratista(Contratista::restaurar(ContratistaGuardado {
            id: ContratistaId::desde_uuid(Uuid::from_u128(700)),
            cedula: Cedula::normalizar("1-1111-1111").unwrap(),
            nombre: NombrePersona::nuevo("ana").unwrap(),
            empresa: acme,
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: "2026-10-01".parse().unwrap(),
            tiene_acceso: true,
        }));
        for (cedula, nombre, vence) in [
            ("2-2222-2222", "beto", "2026-11-08"),
            ("3-3333-3333", "carlos", "2026-11-09"),
        ] {
            app.contratistas
                .registrar
                .ejecutar(
                    &sesion,
                    &ComandoContratista {
                        cedula: cedula.into(),
                        nombre: nombre.into(),
                        empresa: acme,
                        tipo_ingreso: TipoIngreso::Praind,
                        fecha_vencimiento_praind: vence.parse().unwrap(),
                        tiene_acceso: true,
                    },
                )
                .await
                .unwrap();
        }
        let por_vencer = app.contratistas.praind_por_vencer.ejecutar().await.unwrap();
        let nombres: Vec<&str> = por_vencer.iter().map(|c| c.nombre().as_str()).collect();
        assert_eq!(
            nombres,
            ["ANA", "BETO"],
            "ANA ya venció y BETO vence en 30 días; CARLOS en 31"
        );
    }
}
