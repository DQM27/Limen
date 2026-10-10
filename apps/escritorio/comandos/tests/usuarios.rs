//! Los comandos de usuarios e inicio de sesión, con la base en memoria: la
//! sesión del equipo, lo que entra y sale en JSON y el campo de cada error.

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use limen_composicion::Aplicacion;
    use limen_escritorio_comandos::{Comandos, ErrorJson, UsuarioEntrada, campos};
    use limen_infra_memoria::{AlmacenMemoria, ContrasenasFalsas, IdsSecuenciales, RelojFijo};
    use serde_json::json;

    type Prueba = Comandos<AlmacenMemoria, RelojFijo, IdsSecuenciales, ContrasenasFalsas>;

    /// Comandos sobre un equipo recién instalado: sin usuarios ni sesión.
    fn comandos() -> Prueba {
        let almacen = AlmacenMemoria::new();
        let reloj = RelojFijo::new(
            "2026-10-10T14:00:00Z".parse().unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
        );
        Comandos::new(Aplicacion::nueva(
            &almacen,
            &reloj,
            &almacen.ids(),
            &ContrasenasFalsas,
        ))
    }

    fn formulario(cedula: &str, nombre: &str, contrasena: &str) -> UsuarioEntrada {
        UsuarioEntrada {
            cedula: cedula.into(),
            nombre: nombre.into(),
            contrasena: contrasena.into(),
        }
    }

    /// Un equipo con Ana como primer usuario y su sesión abierta.
    async fn con_ana() -> Prueba {
        let comandos = comandos();
        comandos
            .crear_primer_usuario(&formulario("1-1111-1111", "ana mora", "portería segura"))
            .await
            .unwrap();
        comandos
    }

    fn codigo_y_campo(error: &ErrorJson) -> (&'static str, Option<&'static str>) {
        (error.codigo, error.campo)
    }

    #[tokio::test]
    async fn sin_sesion_no_se_hace_nada() {
        let comandos = comandos();
        assert!(!comandos.hay_usuarios().await.unwrap(), "recién instalado");
        assert_eq!(comandos.usuario_actual(), None);
        assert_eq!(comandos.sesion(), Err(ErrorJson::sin_sesion()));
        assert_eq!(
            comandos.cambiar_contrasena("x", "y").await,
            Err(ErrorJson::sin_sesion())
        );
    }

    #[tokio::test]
    async fn el_primer_usuario_abre_la_sesion() {
        let comandos = comandos();
        let ana = comandos
            .crear_primer_usuario(&formulario("1-1111-1111", "ana mora", "portería segura"))
            .await
            .unwrap();
        let json = serde_json::to_value(&ana).unwrap();
        assert_eq!(
            json,
            json!({
                "id": ana.id,
                "cedula": "111111111",
                "nombre": "ANA MORA",
                "debe_cambiar_contrasena": false,
            }),
            "la forma exacta del JSON"
        );
        assert_eq!(comandos.usuario_actual(), Some(ana.clone()));
        assert_eq!(
            comandos.sesion().unwrap().operador().uuid().to_string(),
            ana.id,
            "todo queda a nombre de Ana"
        );
        assert!(comandos.hay_usuarios().await.unwrap(), "ya hay usuarios");

        let segundo = comandos
            .crear_primer_usuario(&formulario("222222222", "beto solís", "otra clave"))
            .await;
        assert_eq!(
            segundo.map_err(|error| codigo_y_campo(&error)),
            Err(("ya_hay_usuarios", None))
        );
    }

    #[tokio::test]
    async fn entra_y_sale() {
        let comandos = con_ana().await;
        comandos.cerrar_sesion();
        assert_eq!(comandos.usuario_actual(), None);
        assert_eq!(comandos.sesion(), Err(ErrorJson::sin_sesion()));

        let ana = comandos
            .iniciar_sesion("111111111", "portería segura")
            .await
            .unwrap();
        assert_eq!(ana.nombre, "ANA MORA");
        assert!(comandos.sesion().is_ok(), "con sesión se opera");
    }

    #[tokio::test]
    async fn el_error_al_entrar_no_dice_que_estaba_mal() {
        let comandos = con_ana().await;
        comandos.cerrar_sesion();
        for (cedula, contrasena) in [
            ("111111111", "equivocada"),
            ("999999999", "portería segura"),
        ] {
            let error = comandos
                .iniciar_sesion(cedula, contrasena)
                .await
                .unwrap_err();
            assert_eq!(
                codigo_y_campo(&error),
                ("credenciales_invalidas", None),
                "sin campo: no revela cuál falló"
            );
            assert_eq!(error.mensaje, "Cédula o contraseña incorrecta");
        }
        assert_eq!(comandos.usuario_actual(), None, "no abrió sesión");
    }

    #[tokio::test]
    async fn con_contrasena_temporal_solo_se_puede_cambiarla() {
        let comandos = con_ana().await;
        let sesion_ana = comandos.sesion().unwrap();
        comandos
            .registrar_usuario(
                &sesion_ana,
                &formulario("222222222", "beto solís", "temporal 123"),
            )
            .await
            .unwrap();
        comandos.cerrar_sesion();

        let beto = comandos
            .iniciar_sesion("222222222", "temporal 123")
            .await
            .unwrap();
        assert!(beto.debe_cambiar_contrasena, "la eligió Ana");
        assert_eq!(
            comandos.sesion().map_err(|error| codigo_y_campo(&error)),
            Err(("contrasena_temporal", None)),
            "no opera hasta cambiarla"
        );

        let error = comandos
            .cambiar_contrasena("no es", "la mía de verdad")
            .await
            .unwrap_err();
        assert_eq!(
            codigo_y_campo(&error),
            (
                "contrasena_actual_incorrecta",
                Some(campos::CONTRASENA_ACTUAL)
            )
        );
        comandos
            .cambiar_contrasena("temporal 123", "la mía de verdad")
            .await
            .unwrap();
        assert!(comandos.sesion().is_ok(), "ya puede operar");
        assert_eq!(
            comandos
                .usuario_actual()
                .map(|usuario| usuario.debe_cambiar_contrasena),
            Some(false)
        );
    }

    #[tokio::test]
    async fn administrar_usuarios_con_el_campo_de_cada_error() {
        let comandos = con_ana().await;
        let sesion = comandos.sesion().unwrap();

        let casos = [
            (
                formulario("", "beto solís", "temporal 123"),
                ("cedula_vacia", campos::CEDULA),
            ),
            (
                formulario("1-1111-1111", "beto solís", "temporal 123"),
                ("usuario_cedula_repetida", campos::CEDULA),
            ),
            (
                formulario("222222222", "", "temporal 123"),
                ("nombre_vacio", campos::NOMBRE),
            ),
            (
                formulario("222222222", "beto solís", "corta"),
                ("contrasena_corta", campos::CONTRASENA),
            ),
            (
                formulario("222222222", "beto solís", "222222222"),
                ("contrasena_igual_a_la_cedula", campos::CONTRASENA),
            ),
        ];
        for (entrada, (codigo, campo)) in casos {
            let error = comandos
                .registrar_usuario(&sesion, &entrada)
                .await
                .unwrap_err();
            assert_eq!(codigo_y_campo(&error), (codigo, Some(campo)), "{entrada:?}");
        }

        let beto = comandos
            .registrar_usuario(
                &sesion,
                &formulario("222222222", "beto solís", "temporal 123"),
            )
            .await
            .unwrap();
        let lista = comandos.listar_usuarios().await.unwrap();
        assert_eq!(
            serde_json::to_value(&lista).unwrap(),
            json!([
                {
                    "id": comandos.usuario_actual().unwrap().id,
                    "cedula": "111111111",
                    "nombre": "ANA MORA",
                    "activo": true,
                    "debe_cambiar_contrasena": false,
                },
                {
                    "id": beto,
                    "cedula": "222222222",
                    "nombre": "BETO SOLIS",
                    "activo": true,
                    "debe_cambiar_contrasena": true,
                },
            ]),
            "por nombre y nunca con la contraseña"
        );

        let cambios = comandos
            .editar_usuario(&sesion, &beto, "beto solís", false)
            .await
            .unwrap();
        assert_eq!(cambios.len(), 1, "sólo cambió si está activo: {cambios:?}");

        let propio = comandos.usuario_actual().unwrap().id;
        let error = comandos
            .editar_usuario(&sesion, &propio, "ana mora", false)
            .await
            .unwrap_err();
        assert_eq!(
            codigo_y_campo(&error),
            ("no_se_desactiva_a_si_mismo", Some(campos::ACTIVO))
        );

        comandos
            .restablecer_contrasena(&sesion, &beto, "nueva temporal")
            .await
            .unwrap();
        let error = comandos
            .restablecer_contrasena(&sesion, "no-es-un-id", "nueva temporal")
            .await
            .unwrap_err();
        assert_eq!(
            error.tipo,
            limen_escritorio_comandos::TipoErrorJson::Negocio
        );
    }
}
