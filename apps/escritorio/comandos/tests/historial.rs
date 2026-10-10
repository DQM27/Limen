//! Los comandos del historial de ingresos, con la base en memoria: el JSON
//! de cada fila, el rango y los accesos rápidos de fecha.

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_composicion::Aplicacion;
    use limen_dominio::gafete::TipoGafete;
    use limen_escritorio_comandos::{
        Comandos, ContratistaEntrada, EntradaContratistaEntrada, campos,
    };
    use limen_infra_memoria::{AlmacenMemoria, ContrasenasFalsas, IdsSecuenciales, RelojFijo};
    use serde_json::json;
    use uuid::Uuid;

    type Prueba = Comandos<AlmacenMemoria, RelojFijo, IdsSecuenciales, ContrasenasFalsas>;

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    /// Comandos con el reloj fijo el viernes 9 de octubre de 2026 a las 08:00
    /// de Costa Rica, y los gafetes de contratista 1 a 5.
    async fn comandos() -> Prueba {
        let almacen = AlmacenMemoria::new();
        let reloj = RelojFijo::new(
            "2026-10-09T14:00:00Z".parse().unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        );
        let app = Aplicacion::nueva(&almacen, &reloj, &almacen.ids(), &ContrasenasFalsas);
        app.gafetes
            .registrar
            .ejecutar(&sesion(), TipoGafete::Contratista, 1, 5)
            .await
            .unwrap();
        Comandos::new(app)
    }

    /// Un contratista entra con el gafete 3 y sale. Devuelve el ingreso.
    async fn entra_y_sale(comandos: &Prueba) -> String {
        let empresa = comandos
            .registrar_empresa(&sesion(), "acme s.a.")
            .await
            .unwrap();
        let contratista = comandos
            .registrar_contratista(
                &sesion(),
                &ContratistaEntrada {
                    cedula: "1-1111-1111".into(),
                    nombre: "josé peña".into(),
                    empresa_id: empresa,
                    tipo_ingreso: "PRAIND".into(),
                    fecha_vencimiento_praind: "2099-01-01".into(),
                    tiene_acceso: true,
                },
            )
            .await
            .unwrap();
        let registrada = comandos
            .registrar_entrada_contratista(
                &sesion(),
                &EntradaContratistaEntrada {
                    contratista_id: contratista,
                    medio: "VEHICULO".into(),
                    placa: Some("abc-123".into()),
                    gafete: Some(3),
                    sin_gafete: false,
                },
            )
            .await
            .unwrap();
        comandos
            .registrar_salida(&sesion(), "CONTRATISTA", &registrada.ingreso_id)
            .await
            .unwrap();
        registrada.ingreso_id
    }

    #[tokio::test]
    async fn el_historial_trae_cada_fila_con_su_entrada_y_salida() {
        let comandos = comandos().await;
        let ingreso = entra_y_sale(&comandos).await;

        let historial = comandos
            .listar_historial(Some("2026-10-09"), Some("2026-10-09"))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&historial).unwrap(),
            json!({
                "desde": "2026-10-09",
                "hasta": "2026-10-09",
                "movimientos": [{
                    "via": "CONTRATISTA",
                    "ingreso_id": ingreso,
                    "identidad": "111111111",
                    "nombre": "JOSE PEÑA",
                    "procedencia": "ACME S.A.",
                    "medio": "VEHICULO",
                    "placa": "ABC-123",
                    "gafete": 3,
                    "sin_gafete": false,
                    "entrada": "2026-10-09T14:00:00Z",
                    "salida": "2026-10-09T14:00:00Z",
                }],
                "truncado": false,
                "maximo": 20_000,
            }),
            "la forma exacta del JSON"
        );
    }

    #[tokio::test]
    async fn sin_limites_trae_todo_y_fuera_del_rango_nada() {
        let comandos = comandos().await;
        entra_y_sale(&comandos).await;
        let todo = comandos.listar_historial(None, Some("")).await.unwrap();
        assert_eq!(todo.movimientos.len(), 1, "vacío = sin límite");
        assert_eq!((todo.desde, todo.hasta), (None, None));

        let otro_dia = comandos
            .listar_historial(Some("2026-10-10"), None)
            .await
            .unwrap();
        assert!(otro_dia.movimientos.is_empty(), "entró el día anterior");
    }

    #[tokio::test]
    async fn los_errores_del_rango_traen_su_campo() {
        let comandos = comandos().await;
        let invertido = comandos
            .listar_historial(Some("2026-10-09"), Some("2026-10-01"))
            .await
            .unwrap_err();
        assert_eq!(
            (invertido.codigo, invertido.campo),
            ("rango_invertido", Some(campos::HASTA))
        );
        let ilegible = comandos
            .listar_historial(Some("09/10/2026"), None)
            .await
            .unwrap_err();
        assert_eq!(
            (ilegible.codigo, ilegible.campo),
            ("fecha_invalida", Some(campos::DESDE))
        );
    }

    #[tokio::test]
    async fn los_atajos_de_fecha_dan_su_rango_segun_hoy() {
        let comandos = comandos().await;
        let atajos = comandos.atajos_de_fecha();
        let codigos: Vec<&str> = atajos.iter().map(|atajo| atajo.codigo).collect();
        assert_eq!(
            codigos,
            [
                "HOY",
                "AYER",
                "ESTA_SEMANA",
                "SEMANA_PASADA",
                "ESTE_MES",
                "MES_PASADO",
                "ULTIMOS_7_DIAS",
                "ULTIMOS_30_DIAS",
                "ULTIMOS_6_MESES",
                "TODO",
            ],
            "todos, en el orden del menú"
        );
        let Some(semana) = atajos.iter().find(|atajo| atajo.codigo == "ESTA_SEMANA") else {
            panic!("está la semana");
        };
        assert_eq!(
            serde_json::to_value(semana).unwrap(),
            json!({
                "codigo": "ESTA_SEMANA",
                "etiqueta": "Esta semana",
                "corta": "Semana",
                "desde": "2026-10-05",
                "hasta": "2026-10-09",
                "por_omision": false,
            }),
            "la semana arranca el lunes"
        );
        let por_omision: Vec<&str> = atajos
            .iter()
            .filter(|atajo| atajo.por_omision)
            .map(|atajo| atajo.codigo)
            .collect();
        assert_eq!(
            por_omision,
            ["ULTIMOS_6_MESES"],
            "uno solo abre el historial"
        );
    }
}
