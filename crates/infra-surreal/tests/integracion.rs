//! Pruebas de integración propias de `SurrealDB`: lo que la batería de
//! contrato no cubre (casos de uso completos contra la base real,
//! auditoría guardada, persistencia en disco y reaplicación del esquema).

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;
    use limen_aplicacion::casos_de_uso::contratistas::{
        ComandoContratista, ConsultarContratista, EditarContratista, RegistrarContratista,
    };
    use limen_aplicacion::casos_de_uso::empresas::RegistrarEmpresa;
    use limen_aplicacion::casos_de_uso::gafetes::{CambiarGafete, CambioGafete, RegistrarGafetes};
    use limen_aplicacion::casos_de_uso::ingresos::{
        ComandoEntrada, RegistrarEntrada, RegistrarSalida,
    };
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::RegistroAuditado;
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::acceso::ResultadoAcceso;
    use limen_dominio::contratista::ErrorContratista;
    use limen_dominio::empresa::EmpresaId;
    use limen_dominio::gafete::{Deudor, ErrorPrestamoGafete, NumeroGafete, TipoGafete};
    use limen_dominio::ingreso_contratista::ErrorIngreso;
    use limen_dominio::medio::TipoMedio;
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_infra_memoria::{IdsSecuenciales, RelojFijo};
    use limen_infra_surreal::AlmacenSurreal;
    use uuid::Uuid;

    fn reloj() -> RelojFijo {
        let hoy = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        RelojFijo::new(hoy.and_hms_opt(15, 0, 0).unwrap().and_utc(), hoy)
    }

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    fn comando(empresa: EmpresaId) -> ComandoContratista {
        ComandoContratista {
            cedula: "1-1111-1111".into(),
            nombre: "josé peña".into(),
            empresa,
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            tiene_acceso: true,
        }
    }

    /// Registra una empresa y un contratista con los casos de uso reales.
    async fn registrar_todo(
        almacen: &AlmacenSurreal,
        ids: &IdsSecuenciales,
    ) -> (EmpresaId, limen_dominio::contratista::ContratistaId) {
        let empresa = RegistrarEmpresa::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), "acme")
            .await
            .unwrap();
        let contratista = RegistrarContratista::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), &comando(empresa))
            .await
            .unwrap();
        (empresa, contratista)
    }

    /// Carpeta temporal única para una base en disco; se borra al soltarla.
    struct CarpetaTemporal(PathBuf);

    impl CarpetaTemporal {
        fn nueva() -> Self {
            Self(std::env::temp_dir().join(format!("limen-prueba-{}", Uuid::now_v7())))
        }
    }

    impl Drop for CarpetaTemporal {
        fn drop(&mut self) {
            drop(std::fs::remove_dir_all(&self.0));
        }
    }

    /// El motor libera el archivo en segundo plano después de soltar la
    /// última conexión: se reintenta mientras el candado siga tomado.
    async fn reabrir(ruta: &std::path::Path) -> AlmacenSurreal {
        for _ in 0..100 {
            match AlmacenSurreal::en_disco(ruta).await {
                Ok(almacen) => return almacen,
                // "locked" en Linux; "os error 33" en Windows.
                Err(error)
                    if error.to_string().contains("locked")
                        || error.to_string().contains("os error 33") =>
                {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                Err(error) => panic!("no se pudo reabrir la base: {error}"),
            }
        }
        panic!("el motor nunca liberó el archivo de la base");
    }

    #[tokio::test]
    async fn los_casos_de_uso_funcionan_contra_surrealdb_y_auditan() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let ids = IdsSecuenciales::new();
        let (empresa, id) = registrar_todo(&almacen, &ids).await;

        let mut cambio = comando(empresa);
        cambio.tiene_acceso = false;
        let cambios = EditarContratista::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), id, &cambio)
            .await
            .unwrap();
        assert_eq!(cambios.len(), 1, "sólo cambió el acceso");

        let historial = almacen
            .auditoria_de(RegistroAuditado::Contratista(id))
            .await
            .unwrap();
        let acciones: Vec<_> = historial.iter().map(|e| e.accion.as_str()).collect();
        assert_eq!(
            acciones,
            ["alta", "edicion"],
            "el alta y la edición quedaron"
        );
        assert_eq!(historial[0].cambios.len(), 6, "el alta guarda cada campo");
        assert_eq!(historial[1].cambios[0].campo, "tiene_acceso");
        assert_eq!(historial[1].cambios[0].antes, "true");
        assert_eq!(historial[1].cambios[0].despues, "false");
        assert_eq!(historial[1].operador, sesion().operador().uuid(), "quién");
    }

    #[tokio::test]
    async fn la_cedula_repetida_llega_al_operador_como_error_de_negocio() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let ids = IdsSecuenciales::new();
        let (empresa, _) = registrar_todo(&almacen, &ids).await;

        let mut misma_persona = comando(empresa);
        misma_persona.cedula = "0111111111".into();
        assert_eq!(
            RegistrarContratista::new(almacen.clone(), reloj(), ids.clone())
                .ejecutar(&sesion(), &misma_persona)
                .await,
            Err(ErrorCaso::Negocio(ErrorContratista::CedulaRepetida))
        );
    }

    #[tokio::test]
    async fn los_datos_persisten_en_disco_y_el_esquema_se_reaplica_al_reabrir() {
        let carpeta = CarpetaTemporal::nueva();
        let ids = IdsSecuenciales::new();
        {
            let almacen = AlmacenSurreal::en_disco(&carpeta.0).await.unwrap();
            registrar_todo(&almacen, &ids).await;
        }

        // Reabrir aplica el esquema otra vez (OVERWRITE) sin perder datos.
        let reabierto = reabrir(&carpeta.0).await;
        let ficha = ConsultarContratista::new(reabierto, reloj())
            .ejecutar("111111111")
            .await
            .unwrap();
        assert_eq!(ficha.contratista.nombre().as_str(), "JOSE PEÑA");
        assert_eq!(ficha.acceso, ResultadoAcceso::Permitido, "PRAIND vigente");
    }

    fn entrada_con_gafete(
        contratista: limen_dominio::contratista::ContratistaId,
    ) -> ComandoEntrada {
        ComandoEntrada {
            contratista,
            medio: TipoMedio::Vehiculo,
            placa: Some("abc-123".into()),
            gafete: Some(7),
        }
    }

    #[tokio::test]
    async fn entrada_y_salida_por_gafete_contra_surrealdb() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let ids = IdsSecuenciales::new();
        let (_, contratista) = registrar_todo(&almacen, &ids).await;
        RegistrarGafetes::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), TipoGafete::Contratista, 1, 10)
            .await
            .unwrap();
        let entrar = RegistrarEntrada::new(almacen.clone(), reloj(), ids.clone());

        let entrada = entrar
            .ejecutar(&sesion(), &entrada_con_gafete(contratista))
            .await
            .unwrap();
        assert_eq!(entrada.acceso, ResultadoAcceso::Permitido);

        // Adentro y con el gafete prestado: no puede entrar otra vez.
        assert_eq!(
            entrar
                .ejecutar(&sesion(), &entrada_con_gafete(contratista))
                .await,
            Err(ErrorCaso::Negocio(ErrorIngreso::YaEstaAdentro(
                limen_dominio::presencia::YaEstaAdentro(limen_dominio::presencia::Via::Contratista)
            )))
        );

        RegistrarSalida::new(almacen.clone(), reloj(), ids.clone())
            .por_gafete(&sesion(), 7)
            .await
            .unwrap();
        // Ya salió: el gafete quedó libre y la persona puede volver a entrar.
        assert_eq!(
            RegistrarSalida::new(almacen.clone(), reloj(), ids.clone())
                .por_gafete(&sesion(), 7)
                .await,
            Err(ErrorCaso::NoEncontrado),
            "no queda ningún ingreso abierto con ese gafete"
        );
        entrar
            .ejecutar(&sesion(), &entrada_con_gafete(contratista))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn un_gafete_perdido_no_se_presta_y_el_cambio_queda_auditado() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let ids = IdsSecuenciales::new();
        let (_, contratista) = registrar_todo(&almacen, &ids).await;
        RegistrarGafetes::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), TipoGafete::Contratista, 7, 7)
            .await
            .unwrap();
        CambiarGafete::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(
                &sesion(),
                TipoGafete::Contratista,
                7,
                CambioGafete::MarcarPerdido(Deudor::Contratista(contratista)),
            )
            .await
            .unwrap();

        let resultado = RegistrarEntrada::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), &entrada_con_gafete(contratista))
            .await;
        assert!(
            matches!(
                resultado,
                Err(ErrorCaso::Negocio(ErrorIngreso::Gafete(
                    ErrorPrestamoGafete::NoDisponible(_)
                )))
            ),
            "{resultado:?}"
        );

        let gafete =
            RegistroAuditado::Gafete(TipoGafete::Contratista, NumeroGafete::nuevo(7).unwrap());
        let historial = almacen.auditoria_de(gafete).await.unwrap();
        let acciones: Vec<_> = historial.iter().map(|e| e.accion.as_str()).collect();
        assert_eq!(acciones, ["alta", "edicion"]);
        assert_eq!(historial[0].registro, "CONTRATISTA-7");
    }

    #[tokio::test]
    async fn un_proveedor_entra_y_sale_contra_surrealdb_y_respeta_el_veto() {
        use limen_aplicacion::casos_de_uso::proveedores::{
            ComandoEntradaProveedor, RegistrarEmpresaProveedora, RegistrarEntradaProveedor,
            RegistrarSalidaProveedor,
        };
        use limen_dominio::ingreso_proveedor::ErrorIngresoProveedor;
        use limen_dominio::visitante::PersonaVetada;

        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let ids = IdsSecuenciales::new();
        let empresa = RegistrarEmpresaProveedora::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), "gas zeta")
            .await
            .unwrap();
        RegistrarGafetes::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), TipoGafete::Proveedor, 1, 5)
            .await
            .unwrap();
        let entrar = RegistrarEntradaProveedor::new(almacen.clone(), reloj(), ids.clone());
        let proveedor = ComandoEntradaProveedor {
            cedula: "2-2222-2222".into(),
            nombre: "maría solís".into(),
            empresa,
            medio: TipoMedio::APie,
            placa: None,
            gafete: 3,
        };

        entrar.ejecutar(&sesion(), &proveedor).await.unwrap();
        RegistrarSalidaProveedor::new(almacen.clone(), reloj(), ids.clone())
            .por_gafete(&sesion(), 3)
            .await
            .unwrap();

        // Un contratista sin acceso tampoco entra como proveedor (A8).
        let (contratistas, contratista) = registrar_todo(&almacen, &ids).await;
        let mut sin_acceso = comando(contratistas);
        sin_acceso.tiene_acceso = false;
        EditarContratista::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), contratista, &sin_acceso)
            .await
            .unwrap();
        let mut vetado = proveedor.clone();
        vetado.cedula = "1-1111-1111".into();
        assert_eq!(
            entrar.ejecutar(&sesion(), &vetado).await,
            Err(ErrorCaso::Negocio(ErrorIngresoProveedor::AccesoDenegado(
                PersonaVetada
            )))
        );

        let historial = almacen
            .auditoria_de(RegistroAuditado::EmpresaProveedora(empresa))
            .await
            .unwrap();
        assert_eq!(historial.len(), 1, "el alta de la empresa proveedora");
        assert_eq!(historial[0].entidad, "empresa_proveedora");
    }

    #[tokio::test]
    async fn una_visita_por_correo_entra_y_sale_contra_surrealdb() {
        use limen_aplicacion::casos_de_uso::correo::{
            ComandoEntradaCorreo, RegistrarEntradaCorreo, RegistrarSalidaCorreo,
        };
        use limen_dominio::ingreso_correo::{ErrorIngresoCorreo, ErrorMotivo};
        use limen_dominio::presencia::{Via, YaEstaAdentro};

        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let ids = IdsSecuenciales::new();
        RegistrarGafetes::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), TipoGafete::Visita, 1, 3)
            .await
            .unwrap();
        let entrar = RegistrarEntradaCorreo::new(almacen.clone(), reloj(), ids.clone());
        let visita = ComandoEntradaCorreo {
            cedula: "3-3333-3333".into(),
            nombre: "luis mora".into(),
            motivo: "Entrevista con RH".into(),
            medio: TipoMedio::APie,
            placa: None,
            gafete: 2,
        };

        let mut sin_motivo = visita.clone();
        sin_motivo.motivo = " ".into();
        assert_eq!(
            entrar.ejecutar(&sesion(), &sin_motivo).await,
            Err(ErrorCaso::Negocio(ErrorIngresoCorreo::Motivo(
                ErrorMotivo::Vacio
            )))
        );

        entrar.ejecutar(&sesion(), &visita).await.unwrap();
        assert_eq!(
            entrar.ejecutar(&sesion(), &visita).await,
            Err(ErrorCaso::Negocio(ErrorIngresoCorreo::YaEstaAdentro(
                YaEstaAdentro(Via::Correo)
            ))),
            "ya está adentro"
        );
        RegistrarSalidaCorreo::new(almacen.clone(), reloj(), ids.clone())
            .por_gafete(&sesion(), 2)
            .await
            .unwrap();
        entrar.ejecutar(&sesion(), &visita).await.unwrap();
    }

    #[tokio::test]
    async fn el_gafete_provisional_kof_se_entrega_y_se_devuelve_contra_surrealdb() {
        use limen_aplicacion::casos_de_uso::kof::{
            DevolverGafeteKof, EntregarGafeteKof, RegistrarPersonalKof,
        };
        use limen_dominio::personal_kof::ErrorPersonalKof;
        use limen_dominio::prestamo_kof::ErrorPrestamoKof;

        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let ids = IdsSecuenciales::new();
        RegistrarGafetes::new(almacen.clone(), reloj(), ids.clone())
            .ejecutar(&sesion(), TipoGafete::ProvisionalKof, 1, 3)
            .await
            .unwrap();
        let registrar = RegistrarPersonalKof::new(almacen.clone(), reloj(), ids.clone());
        let ana = registrar
            .ejecutar(&sesion(), "5040017", "ana mora")
            .await
            .unwrap();
        assert_eq!(
            registrar.ejecutar(&sesion(), "5040017", "otra").await,
            Err(ErrorCaso::Negocio(ErrorPersonalKof::CodigoRepetido))
        );

        let entregar = EntregarGafeteKof::new(almacen.clone(), reloj(), ids.clone());
        entregar.ejecutar(&sesion(), ana, 2).await.unwrap();
        assert_eq!(
            entregar.ejecutar(&sesion(), ana, 3).await,
            Err(ErrorCaso::Negocio(ErrorPrestamoKof::YaTienePrestamo))
        );
        DevolverGafeteKof::new(almacen.clone(), reloj(), ids.clone())
            .por_gafete(&sesion(), 2)
            .await
            .unwrap();
        entregar.ejecutar(&sesion(), ana, 3).await.unwrap();

        let historial = almacen
            .auditoria_de(RegistroAuditado::PersonalKof(ana))
            .await
            .unwrap();
        assert_eq!(historial.len(), 1, "el alta");
        assert_eq!(historial[0].entidad, "personal_kof");
    }
}
