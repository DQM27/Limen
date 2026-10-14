//! Pruebas de la regla A9 en el caso de uso: antes de registrar una entrada
//! se pregunta a la nube si la persona ya está adentro en otra unidad.

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::{Arc, Mutex};

    use chrono::{DateTime, Utc};
    use limen_aplicacion::casos_de_uso::otra_unidad::VerificarOtraUnidad;
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::PresenciaRemota;
    use limen_dominio::contratista::ContratistaId;
    use limen_dominio::origen::UnidadOperativaId;
    use limen_dominio::personal_kof::PersonalKofId;
    use limen_dominio::presencia::{AdentroEnOtraUnidad, Identidad, Via};
    use limen_infra_memoria::AlmacenMemoria;
    use uuid::Uuid;

    /// La nube de mentira: responde lo que se le dijo y anota a quién le preguntaron.
    #[derive(Clone, Default)]
    struct NubeFalsa {
        respuesta: Option<AdentroEnOtraUnidad>,
        preguntas: Arc<Mutex<Vec<String>>>,
    }

    impl PresenciaRemota for NubeFalsa {
        fn consultar(
            &self,
            identidad: &Identidad,
        ) -> impl Future<Output = Option<AdentroEnOtraUnidad>> + Send {
            self.preguntas.lock().unwrap().push(identidad.clave());
            std::future::ready(self.respuesta.clone())
        }
    }

    fn adentro_en(unidad: &str) -> AdentroEnOtraUnidad {
        AdentroEnOtraUnidad {
            unidad: UnidadOperativaId::nueva(unidad).unwrap(),
            via: Via::Contratista,
            desde: "2026-10-14T14:12:00Z".parse::<DateTime<Utc>>().unwrap(),
        }
    }

    fn verificador(nube: NubeFalsa) -> VerificarOtraUnidad<AlmacenMemoria, NubeFalsa> {
        VerificarOtraUnidad::new(
            AlmacenMemoria::new(),
            nube,
            UnidadOperativaId::nueva("BRISAS").unwrap(),
        )
    }

    #[tokio::test]
    async fn adentro_en_otra_unidad_no_deja_registrar_e_informa_donde() {
        let nube = NubeFalsa {
            respuesta: Some(adentro_en("CARTAGO")),
            ..NubeFalsa::default()
        };
        let error = verificador(nube)
            .cedula("1-1111-1111")
            .await
            .expect_err("debe bloquear");
        let ErrorCaso::Negocio(otra) = &error else {
            panic!("debía ser un error de negocio: {error:?}");
        };
        assert_eq!(otra.unidad.as_str(), "CARTAGO");
        assert_eq!(error.para_interfaz().codigo, "adentro_en_otra_unidad");
        assert!(error.para_interfaz().mensaje.contains("CARTAGO"));
    }

    #[tokio::test]
    async fn sin_respuesta_de_la_nube_el_acceso_no_se_bloquea() {
        let nube = NubeFalsa::default();
        assert_eq!(verificador(nube).cedula("1-1111-1111").await, Ok(()));
    }

    #[tokio::test]
    async fn lo_que_la_nube_dice_de_la_propia_unidad_no_bloquea() {
        let nube = NubeFalsa {
            respuesta: Some(adentro_en("BRISAS")),
            ..NubeFalsa::default()
        };
        assert_eq!(verificador(nube).cedula("1-1111-1111").await, Ok(()));
    }

    #[tokio::test]
    async fn pregunta_por_la_cedula_normalizada() {
        let nube = NubeFalsa::default();
        verificador(nube.clone())
            .cedula("1-1111-1111")
            .await
            .unwrap();
        assert_eq!(*nube.preguntas.lock().unwrap(), vec!["111111111"]);
    }

    #[tokio::test]
    async fn una_cedula_mal_escrita_ni_sale_a_la_nube() {
        let nube = NubeFalsa {
            respuesta: Some(adentro_en("CARTAGO")),
            ..NubeFalsa::default()
        };
        assert_eq!(verificador(nube.clone()).cedula("1_2#3").await, Ok(()));
        assert!(nube.preguntas.lock().unwrap().is_empty(), "no preguntó");
    }

    #[tokio::test]
    async fn si_no_existe_el_contratista_o_el_personal_no_pregunta_la_entrada_lo_dira() {
        let nube = NubeFalsa {
            respuesta: Some(adentro_en("CARTAGO")),
            ..NubeFalsa::default()
        };
        let verificador = verificador(nube.clone());
        let inexistente = Uuid::from_u128(77);
        assert_eq!(
            verificador
                .contratista(ContratistaId::desde_uuid(inexistente))
                .await,
            Ok(())
        );
        assert_eq!(
            verificador
                .personal_kof(PersonalKofId::desde_uuid(inexistente))
                .await,
            Ok(())
        );
        assert!(nube.preguntas.lock().unwrap().is_empty(), "no preguntó");
    }
}
