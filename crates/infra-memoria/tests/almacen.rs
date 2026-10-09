//! Pruebas propias del doble en memoria: lo que la batería de contrato no
//! cubre porque no forma parte de los puertos (fallas simuladas, IDs).
//! El comportamiento de persistencia se prueba en `contrato.rs`.

#[cfg(test)]
mod tests {
    use limen_aplicacion::puertos::{
        ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds, RepositorioEmpresas,
        UnidadDeTrabajo,
    };
    use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales};
    use uuid::Uuid;

    fn empresa(id: u128, nombre: &str) -> Empresa {
        Empresa::restaurar(
            EmpresaId::desde_uuid(Uuid::from_u128(id)),
            NombreEmpresa::nuevo(nombre).unwrap(),
        )
    }

    #[tokio::test]
    async fn la_falla_simulada_no_aplica_nada_y_es_de_una_sola_vez() {
        let almacen = AlmacenMemoria::new();
        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Tecnica("disco".into()));
        let mut uow = almacen.nueva();
        uow.empresas().guardar(&empresa(1, "ACME"));
        assert!(uow.confirmar().await.is_err(), "la primera falla");
        assert!(almacen.empresas().is_empty(), "sin aplicar nada");

        let mut otra = almacen.nueva();
        otra.empresas().guardar(&empresa(1, "ACME"));
        assert_eq!(otra.confirmar().await, Ok(()), "la siguiente funciona");
    }

    #[tokio::test]
    async fn las_lecturas_pueden_simular_fallas() {
        let almacen = AlmacenMemoria::new();
        almacen.fallar_lecturas(ErrorPersistencia::Tecnica("caida".into()));
        let mut uow = almacen.nueva();
        let resultado = uow
            .empresas()
            .existe(EmpresaId::desde_uuid(Uuid::nil()))
            .await;
        assert_eq!(resultado, Err(ErrorPersistencia::Tecnica("caida".into())));
    }

    #[test]
    fn los_ids_son_predecibles_y_no_se_repiten() {
        let ids = IdsSecuenciales::new();
        let compartido = ids.clone();
        assert_eq!(ids.nuevo(), Uuid::from_u128(1));
        assert_eq!(
            compartido.nuevo(),
            Uuid::from_u128(2),
            "los clones comparten la cuenta"
        );
    }

    #[test]
    fn el_almacen_reparte_ids_sin_repetir_entre_casos_de_uso() {
        let almacen = AlmacenMemoria::new();
        let uno = almacen.ids();
        let otro = almacen.ids();
        assert_ne!(uno.nuevo(), otro.nuevo(), "comparten el mismo contador");
    }
}
