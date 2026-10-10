//! Los comandos de crear y editar de cada módulo (empresas, proveedores,
//! correo, personal KOF y gafetes), con la base en memoria: lo que entra,
//! lo que sale y el campo de cada error.

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_composicion::Aplicacion;
    use limen_escritorio_comandos::{
        CambioGafeteEntrada, Comandos, ContratistaEntrada, EntradaCorreoEntrada,
        EntradaProveedorEntrada, ErrorJson, campos,
    };
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales, RelojFijo};
    use serde_json::json;
    use uuid::Uuid;

    type Prueba = Comandos<AlmacenMemoria, RelojFijo, IdsSecuenciales>;

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    /// Comandos sobre una base vacía, con el reloj fijo el 9 de octubre.
    fn comandos() -> Prueba {
        let almacen = AlmacenMemoria::new();
        let reloj = RelojFijo::new(
            "2026-10-09T14:00:00Z".parse().unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        );
        Comandos::new(Aplicacion::nueva(&almacen, &reloj, &almacen.ids()))
    }

    fn error_de<T: std::fmt::Debug>(resultado: Result<T, ErrorJson>) -> ErrorJson {
        resultado.unwrap_err()
    }

    /// El código y el campo de un error.
    fn codigo_y_campo(error: &ErrorJson) -> (&'static str, Option<&'static str>) {
        (error.codigo, error.campo)
    }

    // --- Empresas ---

    #[tokio::test]
    async fn una_empresa_se_renombra_y_el_nombre_repetido_va_al_campo_nombre() {
        let comandos = comandos();
        let acme = comandos.registrar_empresa(&sesion(), "acme").await.unwrap();
        comandos.registrar_empresa(&sesion(), "zeta").await.unwrap();

        let cambios = comandos
            .renombrar_empresa(&sesion(), &acme, "acme s.a.")
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&cambios).unwrap(),
            json!([{ "campo": "nombre", "antes": "ACME", "despues": "ACME S.A." }])
        );
        let igual = comandos
            .renombrar_empresa(&sesion(), &acme, "ACME S.A.")
            .await
            .unwrap();
        assert_eq!(igual, Vec::new(), "el mismo nombre no cambia nada");

        let repetido = error_de(comandos.renombrar_empresa(&sesion(), &acme, "zeta").await);
        assert_eq!(
            codigo_y_campo(&repetido),
            ("empresa_nombre_repetido", Some("nombre"))
        );
        let inexistente = error_de(
            comandos
                .renombrar_empresa(&sesion(), &Uuid::from_u128(7).to_string(), "otra")
                .await,
        );
        assert_eq!(codigo_y_campo(&inexistente), ("no_encontrado", None));
    }

    // --- Proveedores ---

    fn proveedor(empresa_id: &str) -> EntradaProveedorEntrada {
        EntradaProveedorEntrada {
            cedula: "2-2222-2222".into(),
            nombre: "maría solís".into(),
            empresa_id: empresa_id.into(),
            medio: "VEHICULO".into(),
            placa: Some("abc-123".into()),
            gafete: 3,
        }
    }

    #[tokio::test]
    async fn las_empresas_proveedoras_se_registran_buscan_y_renombran() {
        let comandos = comandos();
        let gas = comandos
            .registrar_empresa_proveedora(&sesion(), "gas zeta")
            .await
            .unwrap();
        let encontradas = comandos
            .buscar_empresas_proveedoras("zeta", 5)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&encontradas).unwrap(),
            json!([{ "id": gas, "nombre": "GAS ZETA" }])
        );
        let cambios = comandos
            .renombrar_empresa_proveedora(&sesion(), &gas, "gas zeta s.a.")
            .await
            .unwrap();
        assert_eq!(cambios.len(), 1, "un cambio: el nombre");
        let repetida = error_de(
            comandos
                .registrar_empresa_proveedora(&sesion(), "GAS ZETA S.A.")
                .await,
        );
        assert_eq!(repetida.campo, Some("nombre"), "{repetida:?}");
    }

    #[tokio::test]
    async fn la_entrada_de_un_proveedor_aparece_dentro() {
        let comandos = comandos();
        comandos
            .registrar_gafetes(&sesion(), "PROVEEDOR", 1, 5)
            .await
            .unwrap();
        let gas = comandos
            .registrar_empresa_proveedora(&sesion(), "gas zeta")
            .await
            .unwrap();
        let ingreso = comandos
            .registrar_entrada_proveedor(&sesion(), &proveedor(&gas))
            .await
            .unwrap();
        let dentro = comandos.dentro().await.unwrap();
        assert_eq!(dentro.len(), 1, "queda adentro");
        assert_eq!(dentro[0].ingreso_id, ingreso);
        assert_eq!(dentro[0].via, "PROVEEDOR");
    }

    #[tokio::test]
    async fn cada_error_del_proveedor_llega_con_su_campo() {
        let comandos = comandos();
        comandos
            .registrar_gafetes(&sesion(), "PROVEEDOR", 1, 5)
            .await
            .unwrap();
        let gas = comandos
            .registrar_empresa_proveedora(&sesion(), "gas zeta")
            .await
            .unwrap();
        let con = |cambio: fn(&mut EntradaProveedorEntrada)| {
            let mut entrada = proveedor(&gas);
            cambio(&mut entrada);
            entrada
        };
        let casos = [
            (con(|e| e.cedula = "12".into()), "cedula_invalida", "cedula"),
            (
                con(|e| e.nombre = "ana 2".into()),
                "nombre_invalido",
                "nombre",
            ),
            (con(|e| e.placa = None), "placa_requerida", "placa"),
            (con(|e| e.gafete = 99), "gafete_no_registrado", "gafete"),
            (con(|e| e.medio = "BICI".into()), "medio_invalido", "medio"),
            (
                con(|e| e.empresa_id = "x".into()),
                "id_invalido",
                "empresa_id",
            ),
            (
                con(|e| e.empresa_id = Uuid::from_u128(5).to_string()),
                "empresa_proveedora_no_existe",
                "empresa_id",
            ),
        ];
        for (entrada, codigo, campo) in casos {
            let error = error_de(
                comandos
                    .registrar_entrada_proveedor(&sesion(), &entrada)
                    .await,
            );
            assert_eq!(codigo_y_campo(&error), (codigo, Some(campo)), "{entrada:?}");
        }
    }

    // --- Ingreso por correo ---

    fn correo() -> EntradaCorreoEntrada {
        EntradaCorreoEntrada {
            cedula: "3-3333-3333".into(),
            nombre: "luis mora".into(),
            motivo: "entrevista con RH".into(),
            medio: "A_PIE".into(),
            placa: None,
            gafete: 2,
        }
    }

    #[tokio::test]
    async fn la_entrada_por_correo_aparece_dentro_y_sus_errores_traen_campo() {
        let comandos = comandos();
        comandos
            .registrar_gafetes(&sesion(), "VISITA", 1, 5)
            .await
            .unwrap();
        let sin_motivo = error_de(
            comandos
                .registrar_entrada_correo(
                    &sesion(),
                    &EntradaCorreoEntrada {
                        motivo: " ".into(),
                        ..correo()
                    },
                )
                .await,
        );
        assert_eq!(sin_motivo.campo, Some("motivo"), "{sin_motivo:?}");

        let ingreso = comandos
            .registrar_entrada_correo(&sesion(), &correo())
            .await
            .unwrap();
        let dentro = comandos.dentro().await.unwrap();
        assert_eq!(dentro[0].ingreso_id, ingreso);
        assert_eq!(dentro[0].via, "CORREO");

        // Ya está adentro: no es de un campo, es un mensaje general.
        let otra_vez = error_de(
            comandos
                .registrar_entrada_correo(&sesion(), &correo())
                .await,
        );
        assert_eq!(codigo_y_campo(&otra_vez), ("ya_esta_adentro", None));
    }

    // --- Personal KOF ---

    #[tokio::test]
    async fn el_personal_kof_se_registra_busca_edita_y_recibe_su_provisional() {
        let comandos = comandos();
        comandos
            .registrar_gafetes(&sesion(), "PROVISIONAL_KOF", 1, 5)
            .await
            .unwrap();
        let ana = comandos
            .registrar_personal_kof(&sesion(), "5040017", "ana mora")
            .await
            .unwrap();
        let encontradas = comandos.buscar_personal_kof("5040", 5).await.unwrap();
        assert_eq!(
            serde_json::to_value(&encontradas).unwrap(),
            json!([{ "id": ana, "codigo_empleado": "5040017", "nombre": "ANA MORA", "activo": true }])
        );

        let prestamo = comandos
            .entregar_gafete_kof(&sesion(), &ana, 3)
            .await
            .unwrap();
        let dentro = comandos.dentro().await.unwrap();
        assert_eq!(dentro[0].ingreso_id, prestamo, "entregar es su entrada");
        assert_eq!(dentro[0].via, "KOF");

        let cambios = comandos
            .editar_personal_kof(&sesion(), &ana, "ana mora", false)
            .await
            .unwrap();
        assert_eq!(cambios.len(), 1, "sólo cambió si está activa: {cambios:?}");
    }

    #[tokio::test]
    async fn cada_error_del_personal_kof_llega_con_su_campo() {
        let comandos = comandos();
        comandos
            .registrar_gafetes(&sesion(), "PROVISIONAL_KOF", 1, 5)
            .await
            .unwrap();
        let ana = comandos
            .registrar_personal_kof(&sesion(), "5040017", "ana mora")
            .await
            .unwrap();
        let repetido = error_de(
            comandos
                .registrar_personal_kof(&sesion(), "5040017", "otra")
                .await,
        );
        assert_eq!(repetido.campo, Some("codigo_empleado"), "{repetido:?}");
        let mal_nombre = error_de(
            comandos
                .registrar_personal_kof(&sesion(), "5040018", "ana 2")
                .await,
        );
        assert_eq!(mal_nombre.campo, Some("nombre"), "{mal_nombre:?}");
        let sin_gafete = error_de(comandos.entregar_gafete_kof(&sesion(), &ana, 99).await);
        assert_eq!(
            codigo_y_campo(&sin_gafete),
            ("gafete_no_registrado", Some("gafete"))
        );
        let persona_ilegible = error_de(comandos.entregar_gafete_kof(&sesion(), "x", 1).await);
        assert_eq!(
            codigo_y_campo(&persona_ilegible),
            ("id_invalido", Some("personal_id"))
        );

        comandos
            .editar_personal_kof(&sesion(), &ana, "ana mora", false)
            .await
            .unwrap();
        let inactiva = error_de(comandos.entregar_gafete_kof(&sesion(), &ana, 1).await);
        assert_eq!(inactiva.campo, Some("personal_id"), "{inactiva:?}");
    }

    // --- Gafetes ---

    #[tokio::test]
    async fn los_gafetes_se_crean_por_rango_y_se_listan_con_su_estado() {
        let comandos = comandos();
        let creados = comandos
            .registrar_gafetes(&sesion(), "CONTRATISTA", 1, 3)
            .await
            .unwrap();
        assert_eq!(creados, 3);
        let lista = comandos.listar_gafetes("CONTRATISTA").await.unwrap();
        assert_eq!(
            serde_json::to_value(&lista[0]).unwrap(),
            json!({
                "tipo": "CONTRATISTA",
                "numero": 1,
                "estado": "DISPONIBLE",
                "prestado": false,
                "deudor_contratista_id": null,
                "deudor_cedula": null,
            })
        );

        let repetidos = error_de(
            comandos
                .registrar_gafetes(&sesion(), "CONTRATISTA", 3, 4)
                .await,
        );
        assert_eq!(codigo_y_campo(&repetidos), ("gafete_repetido", None));
        let al_reves = error_de(
            comandos
                .registrar_gafetes(&sesion(), "CONTRATISTA", 9, 5)
                .await,
        );
        assert_eq!(al_reves.campo, Some("hasta"), "{al_reves:?}");
        let tipo = error_de(comandos.registrar_gafetes(&sesion(), "OTRO", 1, 2).await);
        assert_eq!(
            codigo_y_campo(&tipo),
            ("tipo_gafete_invalido", Some("tipo"))
        );
    }

    #[tokio::test]
    async fn un_gafete_se_pierde_con_su_deudor_se_paga_y_se_da_de_baja() {
        let comandos = comandos();
        comandos
            .registrar_gafetes(&sesion(), "VISITA", 1, 3)
            .await
            .unwrap();
        let perdido = CambioGafeteEntrada {
            cambio: "PERDIDO".into(),
            deudor_contratista_id: None,
            deudor_cedula: Some("3-3333-3333".into()),
        };
        let cambios = comandos
            .cambiar_gafete(&sesion(), "VISITA", 1, &perdido)
            .await
            .unwrap();
        assert!(!cambios.is_empty(), "queda auditado");
        let lista = comandos.listar_gafetes("VISITA").await.unwrap();
        assert_eq!(lista[0].estado, "PERDIDO");
        assert_eq!(lista[0].deudor_cedula.as_deref(), Some("333333333"));

        let pagado = CambioGafeteEntrada {
            cambio: "PAGADO".into(),
            deudor_contratista_id: None,
            deudor_cedula: None,
        };
        comandos
            .cambiar_gafete(&sesion(), "VISITA", 1, &pagado)
            .await
            .unwrap();
        let baja = CambioGafeteEntrada {
            cambio: "DE_BAJA".into(),
            ..pagado.clone()
        };
        comandos
            .cambiar_gafete(&sesion(), "VISITA", 1, &baja)
            .await
            .unwrap();
        assert_eq!(
            comandos.listar_gafetes("VISITA").await.unwrap()[0].estado,
            "DE_BAJA"
        );

        // Lo que no se puede por el estado del gafete es un mensaje general.
        let no_perdido = error_de(
            comandos
                .cambiar_gafete(&sesion(), "VISITA", 2, &pagado)
                .await,
        );
        assert_eq!(codigo_y_campo(&no_perdido), ("gafete_no_perdido", None));
    }

    #[tokio::test]
    async fn un_cambio_de_gafete_ilegible_dice_su_campo() {
        let comandos = comandos();
        comandos
            .registrar_gafetes(&sesion(), "VISITA", 1, 3)
            .await
            .unwrap();
        let sin_deudor = CambioGafeteEntrada {
            cambio: "PERDIDO".into(),
            deudor_contratista_id: None,
            deudor_cedula: None,
        };
        let casos = [
            (sin_deudor.clone(), "deudor_invalido", "deudor"),
            (
                CambioGafeteEntrada {
                    deudor_cedula: Some("¿?".into()),
                    ..sin_deudor.clone()
                },
                "deudor_invalido",
                "deudor",
            ),
            (
                CambioGafeteEntrada {
                    deudor_cedula: Some("111111111".into()),
                    deudor_contratista_id: Some(Uuid::from_u128(1).to_string()),
                    ..sin_deudor.clone()
                },
                "deudor_invalido",
                "deudor",
            ),
            (
                CambioGafeteEntrada {
                    cambio: "ROBADO".into(),
                    ..sin_deudor.clone()
                },
                "cambio_gafete_invalido",
                "cambio",
            ),
        ];
        for (cambio, codigo, campo) in casos {
            let error = error_de(
                comandos
                    .cambiar_gafete(&sesion(), "VISITA", 1, &cambio)
                    .await,
            );
            assert_eq!(codigo_y_campo(&error), (codigo, Some(campo)), "{cambio:?}");
        }
    }

    // --- Campos ---

    #[test]
    fn los_campos_son_las_claves_de_cada_formulario() {
        // Si una constante dejara de coincidir con una clave del JSON de
        // entrada, el formulario no se podría leer.
        let proveedor: EntradaProveedorEntrada = serde_json::from_value(json!({
            campos::CEDULA: "1", campos::NOMBRE: "A", campos::EMPRESA: "e",
            campos::MEDIO: "A_PIE", campos::PLACA: null, campos::GAFETE: 1,
        }))
        .unwrap();
        assert_eq!(proveedor.gafete, 1);
        let correo: EntradaCorreoEntrada = serde_json::from_value(json!({
            campos::CEDULA: "1", campos::NOMBRE: "A", campos::MOTIVO: "m",
            campos::MEDIO: "A_PIE", campos::PLACA: null, campos::GAFETE: 1,
        }))
        .unwrap();
        assert_eq!(correo.motivo, "m");
        let contratista: ContratistaEntrada = serde_json::from_value(json!({
            campos::CEDULA: "1", campos::NOMBRE: "A", campos::EMPRESA: "e",
            campos::TIPO_INGRESO: "PRAIND", campos::FECHA_VENCIMIENTO_PRAIND: "2099-01-01",
            "tiene_acceso": true,
        }))
        .unwrap();
        assert_eq!(contratista.nombre, "A");
    }
}
