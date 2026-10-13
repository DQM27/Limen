//! La conexión con la nube, probada contra una base en memoria (`mem://`):
//! el mismo código que habla con `SurrealDB` Cloud, sin red ni credenciales.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use limen_infra_nube::{
        ConfigNube, Credenciales, ErrorNube, Nivel, Nube, VAR_CLAVE, VAR_NIVEL, VAR_URL,
        VAR_USUARIO,
    };

    fn entorno(pares: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let mapa: HashMap<String, String> = pares
            .iter()
            .map(|(clave, valor)| ((*clave).to_owned(), (*valor).to_owned()))
            .collect();
        move |nombre| mapa.get(nombre).cloned()
    }

    fn completo() -> Vec<(&'static str, &'static str)> {
        vec![
            (VAR_URL, " wss://limen.aws-use1.surreal.cloud "),
            (VAR_USUARIO, "limen"),
            (VAR_CLAVE, " clave con espacios "),
        ]
    }

    #[test]
    fn la_configuracion_sale_del_entorno_y_el_nivel_por_omision_es_root() {
        let config = ConfigNube::desde_entorno(entorno(&completo())).unwrap();
        assert_eq!(config.url, "wss://limen.aws-use1.surreal.cloud");
        assert_eq!(
            config.credenciales,
            Some(Credenciales {
                nivel: Nivel::Root,
                usuario: "limen".to_owned(),
                clave: " clave con espacios ".to_owned(),
            }),
            "la clave no se recorta"
        );
    }

    #[test]
    fn el_nivel_se_puede_elegir() {
        for (texto, nivel) in [
            ("namespace", Nivel::Namespace),
            ("NS", Nivel::Namespace),
            ("base", Nivel::Base),
            ("database", Nivel::Base),
            ("root", Nivel::Root),
        ] {
            let mut pares = completo();
            pares.push((VAR_NIVEL, texto));
            let config = ConfigNube::desde_entorno(entorno(&pares)).unwrap();
            assert_eq!(config.credenciales.map(|c| c.nivel), Some(nivel), "{texto}");
        }
        let mut pares = completo();
        pares.push((VAR_NIVEL, "otro"));
        assert_eq!(
            ConfigNube::desde_entorno(entorno(&pares)),
            Err(ErrorNube::NivelInvalido)
        );
    }

    #[test]
    fn sin_url_usuario_o_clave_no_se_conecta() {
        for falta in [VAR_URL, VAR_USUARIO, VAR_CLAVE] {
            let pares: Vec<_> = completo()
                .into_iter()
                .filter(|(nombre, _)| *nombre != falta)
                .collect();
            assert_eq!(
                ConfigNube::desde_entorno(entorno(&pares)),
                Err(ErrorNube::FaltaVariable(falta)),
                "falta {falta}"
            );
        }
        let mut vacia = completo();
        vacia.push((VAR_USUARIO, "  "));
        let vacia: Vec<_> = vacia
            .into_iter()
            .filter(|par| *par != (VAR_USUARIO, "limen"))
            .collect();
        assert_eq!(
            ConfigNube::desde_entorno(entorno(&vacia)),
            Err(ErrorNube::FaltaVariable(VAR_USUARIO)),
            "un usuario en blanco cuenta como faltante"
        );
    }

    #[test]
    fn la_clave_nunca_sale_en_debug() {
        let config = ConfigNube::desde_entorno(entorno(&completo())).unwrap();
        let texto = format!("{config:?}");
        assert!(!texto.contains("clave con espacios"), "{texto}");
        assert!(texto.contains("***"), "{texto}");
    }

    fn en_memoria() -> ConfigNube {
        ConfigNube {
            url: "mem://".to_owned(),
            credenciales: None,
        }
    }

    #[tokio::test]
    async fn conecta_y_aplica_el_esquema_de_los_equipos() {
        let nube = Nube::conectar(&en_memoria()).await.unwrap();
        assert!(
            nube.estado().await.unwrap().tablas.is_empty(),
            "una base nueva no tiene tablas"
        );

        nube.aplicar_esquema().await.unwrap();

        let estado = nube.estado().await.unwrap();
        for tabla in ["auditoria", "contratista", "hecho", "presencia", "usuario"] {
            assert!(
                estado.tablas.iter().any(|t| t == tabla),
                "falta la tabla {tabla}: {:?}",
                estado.tablas
            );
        }
        assert!(estado.tablas.is_sorted(), "en orden alfabético");
        assert!(!estado.version.is_empty(), "dice su versión");
    }

    #[tokio::test]
    async fn aplicar_el_esquema_otra_vez_no_cambia_nada() {
        let nube = Nube::conectar(&en_memoria()).await.unwrap();
        nube.aplicar_esquema().await.unwrap();
        let antes = nube.estado().await.unwrap();

        nube.aplicar_esquema().await.unwrap();

        assert_eq!(nube.estado().await.unwrap(), antes);
    }

    #[tokio::test]
    async fn una_direccion_invalida_es_error_de_conexion() {
        let config = ConfigNube {
            url: "no-es-una-direccion".to_owned(),
            credenciales: None,
        };
        let error = Nube::conectar(&config).await.unwrap_err();
        assert!(matches!(error, ErrorNube::Conexion(_)), "{error:?}");
    }
}
