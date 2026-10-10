//! Los comandos de la sesión, con la base en memoria: la sesión del equipo,
//! lo que entra y sale en JSON y el campo de cada error. Los usuarios no se
//! crean en el equipo (vienen de la nube): las pruebas los siembran en la base.

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use limen_composicion::Aplicacion;
    use limen_dominio::cedula::Cedula;
    use limen_dominio::nombre::NombrePersona;
    use limen_dominio::operador::OperadorId;
    use limen_dominio::usuario::{Usuario, UsuarioGuardado};
    use limen_escritorio_comandos::{Comandos, ErrorJson, campos};
    use limen_infra_memoria::{AlmacenMemoria, ClavesFalsas, IdsSecuenciales, RelojFijo};
    use serde_json::json;
    use uuid::Uuid;

    type Prueba = Comandos<AlmacenMemoria, RelojFijo, IdsSecuenciales, ClavesFalsas>;

    /// Un usuario como llega de la nube.
    fn usuario(n: u128, cedula: &str, nombre: &str, clave: &str, temporal: bool) -> Usuario {
        Usuario::restaurar(UsuarioGuardado {
            id: OperadorId::desde_uuid(Uuid::from_u128(n)),
            cedula: Cedula::normalizar(cedula).unwrap(),
            nombre: NombrePersona::nuevo(nombre).unwrap(),
            activo: true,
            clave: ClavesFalsas::hash_de(clave),
            debe_cambiar_clave: temporal,
        })
    }

    /// Comandos sin sesión, sobre una base con Ana (clave propia) y Beto
    /// (clave temporal puesta en la nube).
    fn comandos() -> Prueba {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_usuario(usuario(
            1,
            "111111111",
            "ana mora",
            "portería segura",
            false,
        ));
        almacen.sembrar_usuario(usuario(2, "222222222", "beto solís", "temporal 123", true));
        let reloj = RelojFijo::new(
            "2026-10-10T14:00:00Z".parse().unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
        );
        Comandos::new(Aplicacion::nueva(
            &almacen,
            &reloj,
            &almacen.ids(),
            &ClavesFalsas,
        ))
    }

    fn codigo_y_campo(error: &ErrorJson) -> (&'static str, Option<&'static str>) {
        (error.codigo, error.campo)
    }

    #[tokio::test]
    async fn sin_sesion_no_se_hace_nada() {
        let comandos = comandos();
        assert_eq!(comandos.usuario_actual(), None, "la app arranca sin sesión");
        assert_eq!(comandos.sesion(), Err(ErrorJson::sin_sesion()));
        assert_eq!(
            comandos.cambiar_clave("x", "y").await,
            Err(ErrorJson::sin_sesion())
        );
    }

    #[tokio::test]
    async fn entra_y_sale() {
        let comandos = comandos();
        let ana = comandos
            .iniciar_sesion("1-1111-1111", "portería segura")
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&ana).unwrap(),
            json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "cedula": "111111111",
                "nombre": "ANA MORA",
                "debe_cambiar_clave": false,
            }),
            "la forma exacta del JSON"
        );
        assert_eq!(comandos.usuario_actual(), Some(ana.clone()));
        assert_eq!(
            comandos.sesion().unwrap().operador().uuid().to_string(),
            ana.id,
            "todo queda a nombre de Ana"
        );

        comandos.cerrar_sesion();
        assert_eq!(comandos.usuario_actual(), None);
        assert_eq!(comandos.sesion(), Err(ErrorJson::sin_sesion()));
    }

    #[tokio::test]
    async fn el_error_al_entrar_no_dice_que_estaba_mal() {
        let comandos = comandos();
        for (cedula, clave) in [
            ("111111111", "equivocada"),
            ("999999999", "portería segura"),
        ] {
            let error = comandos.iniciar_sesion(cedula, clave).await.unwrap_err();
            assert_eq!(
                codigo_y_campo(&error),
                ("credenciales_invalidas", None),
                "sin campo: no revela cuál falló"
            );
            assert_eq!(error.mensaje, "Cédula o clave incorrecta");
        }
        assert_eq!(comandos.usuario_actual(), None, "no abrió sesión");
    }

    #[tokio::test]
    async fn con_clave_temporal_solo_se_puede_cambiarla() {
        let comandos = comandos();
        let beto = comandos
            .iniciar_sesion("222222222", "temporal 123")
            .await
            .unwrap();
        assert!(beto.debe_cambiar_clave, "la puso la nube");
        assert_eq!(
            comandos.sesion().map_err(|error| codigo_y_campo(&error)),
            Err(("clave_temporal", None)),
            "no opera hasta cambiarla"
        );

        comandos
            .cambiar_clave("temporal 123", "la mía de verdad")
            .await
            .unwrap();
        assert!(comandos.sesion().is_ok(), "ya puede operar");
        assert_eq!(
            comandos
                .usuario_actual()
                .map(|usuario| usuario.debe_cambiar_clave),
            Some(false)
        );
        comandos.cerrar_sesion();
        assert!(
            comandos
                .iniciar_sesion("222222222", "la mía de verdad")
                .await
                .is_ok(),
            "entra con la clave nueva"
        );
    }

    #[tokio::test]
    async fn cambiar_la_clave_trae_el_campo_de_cada_error() {
        let comandos = comandos();
        comandos
            .iniciar_sesion("111111111", "portería segura")
            .await
            .unwrap();
        let casos = [
            (
                ("no es", "la mía de verdad"),
                ("clave_actual_incorrecta", campos::CLAVE_ACTUAL),
            ),
            (("portería segura", "corta"), ("clave_corta", campos::CLAVE)),
            (
                ("portería segura", "1-1111-1111"),
                ("clave_igual_a_la_cedula", campos::CLAVE),
            ),
        ];
        for ((actual, nueva), (codigo, campo)) in casos {
            let error = comandos.cambiar_clave(actual, nueva).await.unwrap_err();
            assert_eq!(
                codigo_y_campo(&error),
                (codigo, Some(campo)),
                "{actual} → {nueva}"
            );
        }
    }
}
