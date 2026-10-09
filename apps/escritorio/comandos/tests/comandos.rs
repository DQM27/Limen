//! Pruebas de los comandos de la interfaz con la base en memoria: lo que
//! entra y sale en JSON, y cómo llega cada tipo de error.

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use limen_aplicacion::casos_de_uso::contratistas::ComandoContratista;
    use limen_aplicacion::casos_de_uso::correo::ComandoEntradaCorreo;
    use limen_aplicacion::casos_de_uso::ingresos::ComandoEntrada;
    use limen_aplicacion::casos_de_uso::proveedores::ComandoEntradaProveedor;
    use limen_aplicacion::errores::{ErrorCaso, MENSAJE_ERROR_TECNICO};
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_composicion::Aplicacion;
    use limen_dominio::contratista::ErrorContratista;
    use limen_dominio::gafete::TipoGafete;
    use limen_dominio::medio::TipoMedio;
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_escritorio_comandos::{
        Comandos, ContratistaEntrada, EntradaContratistaEntrada, ErrorJson, TipoErrorJson,
    };
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales, RelojFijo};
    use serde_json::json;
    use uuid::Uuid;

    type App = Aplicacion<AlmacenMemoria, RelojFijo, IdsSecuenciales>;
    type Prueba = Comandos<AlmacenMemoria, RelojFijo, IdsSecuenciales>;

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    /// Una aplicación en memoria con el reloj fijo el 9 de octubre de 2026,
    /// y gafetes de cada tipo para poder prestar.
    async fn aplicacion() -> App {
        let almacen = AlmacenMemoria::new();
        let reloj = RelojFijo::new(
            "2026-10-09T14:00:00Z".parse().unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        );
        let app = Aplicacion::nueva(&almacen, &reloj, &almacen.ids());
        for tipo in [
            TipoGafete::Contratista,
            TipoGafete::Proveedor,
            TipoGafete::Visita,
            TipoGafete::ProvisionalKof,
        ] {
            app.gafetes
                .registrar
                .ejecutar(&sesion(), tipo, 1, 5)
                .await
                .unwrap();
        }
        app
    }

    fn formulario(empresa_id: &str) -> ContratistaEntrada {
        ContratistaEntrada {
            cedula: "1-1111-1111".into(),
            nombre: "josé peña".into(),
            empresa_id: empresa_id.into(),
            tipo_ingreso: "PRAIND".into(),
            fecha_vencimiento_praind: "2099-01-01".into(),
            tiene_acceso: true,
        }
    }

    fn entrada(contratista_id: &str, gafete: Option<u32>) -> EntradaContratistaEntrada {
        EntradaContratistaEntrada {
            contratista_id: contratista_id.into(),
            medio: "VEHICULO".into(),
            placa: Some("abc-123".into()),
            gafete,
        }
    }

    /// Una empresa y un contratista ya registrados.
    async fn con_contratista() -> (Prueba, String) {
        let comandos = Comandos::new(aplicacion().await);
        let empresa = comandos
            .registrar_empresa(&sesion(), "acme s.a.")
            .await
            .unwrap();
        let contratista = comandos
            .registrar_contratista(&sesion(), &formulario(&empresa))
            .await
            .unwrap();
        (comandos, contratista)
    }

    #[tokio::test]
    async fn el_contratista_se_registra_se_busca_entra_aparece_dentro_y_sale() {
        let comandos = Comandos::new(aplicacion().await);
        let empresa = comandos
            .registrar_empresa(&sesion(), "acme s.a.")
            .await
            .unwrap();

        let empresas = comandos.buscar_empresas("acm", 10).await.unwrap();
        assert_eq!(empresas.len(), 1, "la empresa se encuentra");
        assert_eq!(empresas[0].id, empresa, "con el mismo id");
        assert_eq!(empresas[0].nombre, "ACME S.A.", "en mayúsculas (A6)");

        let contratista = comandos
            .registrar_contratista(&sesion(), &formulario(&empresa))
            .await
            .unwrap();
        let encontrados = comandos.buscar_contratistas("jose pe", 10).await.unwrap();
        assert_eq!(encontrados.len(), 1, "el contratista se encuentra");
        assert_eq!(
            serde_json::to_value(&encontrados[0]).unwrap(),
            json!({
                "id": contratista,
                "cedula": "111111111",
                "nombre": "JOSE PEÑA",
                "empresa_id": empresa,
                "tipo_ingreso": "PRAIND",
                "fecha_vencimiento_praind": "2099-01-01",
                "tiene_acceso": true,
                "requiere_gafete": true,
            }),
            "la forma exacta del JSON"
        );

        let registrada = comandos
            .registrar_entrada_contratista(&sesion(), &entrada(&contratista, Some(3)))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&registrada.acceso).unwrap(),
            json!({ "resultado": "PERMITIDO", "dias_para_vencer": null, "motivo": null }),
            "PRAIND vigente"
        );

        let dentro = comandos.dentro().await.unwrap();
        assert_eq!(
            serde_json::to_value(&dentro).unwrap(),
            json!([{
                "via": "CONTRATISTA",
                "ingreso_id": registrada.ingreso_id,
                "identidad": "111111111",
                "nombre": "JOSE PEÑA",
                "procedencia": "ACME S.A.",
                "medio": "VEHICULO",
                "placa": "ABC-123",
                "gafete": 3,
                "desde": "2026-10-09T14:00:00Z",
            }]),
            "la fila de la lista dentro"
        );

        comandos
            .registrar_salida(&sesion(), "CONTRATISTA", &registrada.ingreso_id)
            .await
            .unwrap();
        assert_eq!(comandos.dentro().await.unwrap().len(), 0, "ya salió");
    }

    #[tokio::test]
    async fn la_grilla_lista_a_todos_con_el_nombre_de_su_empresa() {
        let (comandos, contratista) = con_contratista().await;
        let empresa = comandos
            .buscar_empresas("acme", 1)
            .await
            .unwrap()
            .remove(0)
            .id;

        let filas = comandos.listar_contratistas().await.unwrap();

        assert_eq!(
            serde_json::to_value(&filas).unwrap(),
            json!([{
                "id": contratista,
                "cedula": "111111111",
                "nombre": "JOSE PEÑA",
                "empresa_id": empresa,
                "tipo_ingreso": "PRAIND",
                "fecha_vencimiento_praind": "2099-01-01",
                "tiene_acceso": true,
                "requiere_gafete": true,
                "empresa_nombre": "ACME S.A.",
            }]),
            "el contratista y el nombre de su empresa, en un solo objeto plano"
        );
    }

    #[tokio::test]
    async fn la_salida_por_gafete_cierra_el_ingreso() {
        let (comandos, contratista) = con_contratista().await;
        comandos
            .registrar_entrada_contratista(&sesion(), &entrada(&contratista, Some(2)))
            .await
            .unwrap();
        comandos
            .registrar_salida_por_gafete(&sesion(), "CONTRATISTA", 2)
            .await
            .unwrap();
        assert_eq!(
            comandos.dentro().await.unwrap().len(),
            0,
            "no queda nadie adentro"
        );
    }

    /// Entran por las cuatro vías; devuelve los comandos y los gafetes.
    async fn entran_por_las_cuatro_vias() -> Prueba {
        let app = aplicacion().await;
        let sesion = sesion();
        let acme = app
            .empresas
            .registrar
            .ejecutar(&sesion, "acme")
            .await
            .unwrap();
        let gas = app
            .proveedores
            .registrar_empresa
            .ejecutar(&sesion, "gas zeta")
            .await
            .unwrap();
        let michael = app
            .kof
            .registrar
            .ejecutar(&sesion, "5040017", "michael araya")
            .await
            .unwrap();
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
        app.ingresos
            .entrada
            .ejecutar(
                &sesion,
                &ComandoEntrada {
                    contratista,
                    medio: TipoMedio::APie,
                    placa: None,
                    gafete: Some(3),
                },
            )
            .await
            .unwrap();
        app.proveedores
            .entrada
            .ejecutar(
                &sesion,
                &ComandoEntradaProveedor {
                    cedula: "2-2222-2222".into(),
                    nombre: "beto solís".into(),
                    empresa: gas,
                    medio: TipoMedio::APie,
                    placa: None,
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
            .ejecutar(&sesion, michael, 1)
            .await
            .unwrap();
        Comandos::new(app)
    }

    #[tokio::test]
    async fn la_lista_dentro_junta_las_cuatro_vias_y_cada_una_sale_por_su_ingreso() {
        let comandos = entran_por_las_cuatro_vias().await;
        let dentro = comandos.dentro().await.unwrap();
        let mut vias: Vec<_> = dentro.iter().map(|p| p.via).collect();
        vias.sort_unstable();
        assert_eq!(vias, ["CONTRATISTA", "CORREO", "KOF", "PROVEEDOR"]);
        let kof = dentro.iter().find(|p| p.via == "KOF").unwrap();
        assert_eq!(kof.identidad, "5040017", "el KOF por código de empleado");
        assert_eq!(kof.medio, None, "el KOF no registra medio");

        for persona in &dentro {
            comandos
                .registrar_salida(&sesion(), persona.via, &persona.ingreso_id)
                .await
                .unwrap();
        }
        assert_eq!(
            comandos.dentro().await.unwrap().len(),
            0,
            "no queda nadie adentro"
        );
    }

    #[tokio::test]
    async fn cada_via_sale_tambien_por_su_gafete() {
        let comandos = entran_por_las_cuatro_vias().await;
        for (via, gafete) in [
            ("CONTRATISTA", 3),
            ("PROVEEDOR", 2),
            ("CORREO", 4),
            ("KOF", 1),
        ] {
            comandos
                .registrar_salida_por_gafete(&sesion(), via, gafete)
                .await
                .unwrap();
        }
        assert_eq!(
            comandos.dentro().await.unwrap().len(),
            0,
            "no queda nadie adentro"
        );
    }

    // --- Errores ---

    fn error_de<T: std::fmt::Debug>(resultado: Result<T, ErrorJson>) -> ErrorJson {
        resultado.unwrap_err()
    }

    #[tokio::test]
    async fn una_regla_del_dominio_llega_con_su_codigo_y_su_mensaje() {
        let (comandos, contratista) = con_contratista().await;
        let empresa = comandos
            .buscar_empresas("acme", 1)
            .await
            .unwrap()
            .remove(0)
            .id;

        let repetida = error_de(
            comandos
                .registrar_contratista(&sesion(), &formulario(&empresa))
                .await,
        );
        assert_eq!(repetida.tipo, TipoErrorJson::Negocio);
        assert_eq!(repetida.codigo, "cedula_repetida");
        assert!(!repetida.mensaje.is_empty(), "trae el mensaje de la regla");

        let mut vencido = formulario(&empresa);
        vencido.cedula = "4-4444-4444".into();
        vencido.fecha_vencimiento_praind = "2020-01-01".into();
        let vencido = error_de(comandos.registrar_contratista(&sesion(), &vencido).await);
        assert_eq!(vencido.codigo, "praind_vencido");

        let sin_gafete = error_de(
            comandos
                .registrar_entrada_contratista(&sesion(), &entrada(&contratista, Some(99)))
                .await,
        );
        assert_eq!(sin_gafete.codigo, "gafete_no_registrado");
    }

    #[tokio::test]
    async fn el_acceso_denegado_no_registra_la_entrada() {
        let comandos = Comandos::new(aplicacion().await);
        let empresa = comandos.registrar_empresa(&sesion(), "acme").await.unwrap();
        let mut sin_acceso = formulario(&empresa);
        sin_acceso.tiene_acceso = false;
        let contratista = comandos
            .registrar_contratista(&sesion(), &sin_acceso)
            .await
            .unwrap();
        let error = error_de(
            comandos
                .registrar_entrada_contratista(&sesion(), &entrada(&contratista, None))
                .await,
        );
        assert_eq!(error.codigo, "sin_acceso");
        assert_eq!(
            comandos.dentro().await.unwrap().len(),
            0,
            "no queda nadie adentro"
        );
    }

    #[tokio::test]
    async fn un_dato_ilegible_se_rechaza_antes_de_llamar_al_caso_de_uso() {
        let (comandos, contratista) = con_contratista().await;
        let empresa = comandos
            .buscar_empresas("acme", 1)
            .await
            .unwrap()
            .remove(0)
            .id;
        let sesion = sesion();

        let casos = [
            (
                error_de(
                    comandos
                        .registrar_salida(&sesion, "CONTRATISTA", "no-es-uuid")
                        .await,
                ),
                "id_invalido",
            ),
            (
                error_de(
                    comandos
                        .registrar_salida(&sesion, "OTRA", &contratista)
                        .await,
                ),
                "via_invalida",
            ),
            (
                error_de(
                    comandos
                        .registrar_salida_por_gafete(&sesion, "OTRA", 1)
                        .await,
                ),
                "via_invalida",
            ),
            (
                error_de(
                    comandos
                        .registrar_contratista(
                            &sesion,
                            &ContratistaEntrada {
                                fecha_vencimiento_praind: "09/10/2026".into(),
                                ..formulario(&empresa)
                            },
                        )
                        .await,
                ),
                "fecha_invalida",
            ),
            (
                error_de(
                    comandos
                        .registrar_contratista(
                            &sesion,
                            &ContratistaEntrada {
                                tipo_ingreso: "SWAT".into(),
                                ..formulario(&empresa)
                            },
                        )
                        .await,
                ),
                "tipo_ingreso_invalido",
            ),
            (
                error_de(
                    comandos
                        .registrar_contratista(
                            &sesion,
                            &ContratistaEntrada {
                                empresa_id: "basura".into(),
                                ..formulario(&empresa)
                            },
                        )
                        .await,
                ),
                "id_invalido",
            ),
            (
                error_de(
                    comandos
                        .registrar_entrada_contratista(
                            &sesion,
                            &EntradaContratistaEntrada {
                                medio: "BICI".into(),
                                ..entrada(&contratista, None)
                            },
                        )
                        .await,
                ),
                "medio_invalido",
            ),
        ];
        for (error, codigo) in casos {
            assert_eq!(error.codigo, codigo, "{error:?}");
            assert_eq!(error.tipo, TipoErrorJson::Negocio, "{error:?}");
            assert!(!error.mensaje.is_empty(), "{error:?}");
        }
    }

    #[tokio::test]
    async fn la_salida_de_un_ingreso_que_no_existe_es_no_encontrado() {
        let comandos = Comandos::new(aplicacion().await);
        let inexistente = Uuid::from_u128(12345).to_string();
        for via in ["CONTRATISTA", "PROVEEDOR", "CORREO", "KOF"] {
            let error = error_de(
                comandos
                    .registrar_salida(&sesion(), via, &inexistente)
                    .await,
            );
            assert_eq!(error.codigo, "no_encontrado", "{via}");
        }
    }

    #[test]
    fn una_falla_tecnica_no_deja_pasar_el_detalle_a_la_pantalla() {
        let error: ErrorJson =
            ErrorCaso::<ErrorContratista>::Tecnico("disco lleno en C:\\datos".into()).into();
        assert_eq!(error.tipo, TipoErrorJson::Tecnico);
        assert_eq!(error.codigo, "error_tecnico");
        assert_eq!(error.mensaje, MENSAJE_ERROR_TECNICO);
        let texto = serde_json::to_string(&error).unwrap();
        assert!(!texto.contains("disco"), "el detalle no viaja: {texto}");
    }

    #[test]
    fn sin_operador_es_un_error_de_negocio_con_codigo_propio() {
        let error = ErrorJson::sin_operador();
        assert_eq!(error.tipo, TipoErrorJson::Negocio);
        assert_eq!(error.codigo, "sin_operador");
        assert_ne!(error.mensaje, "", "trae un mensaje para mostrar");
    }

    #[test]
    fn el_error_viaja_con_tres_campos_y_nada_mas() {
        let error: ErrorJson = ErrorCaso::<ErrorContratista>::NoEncontrado.into();
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            json!({
                "tipo": "negocio",
                "codigo": "no_encontrado",
                "mensaje": "El registro no existe",
            })
        );
    }
}
