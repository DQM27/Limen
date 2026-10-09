//! El doble en memoria tiene que comportarse como la base real; si no, las
//! pruebas de los casos de uso pasarían por las razones equivocadas.

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use limen_aplicacion::puertos::{
        ErrorPersistencia, FabricaUnidadDeTrabajo, GeneradorIds, RepositorioContratistas,
        RepositorioEmpresas, Restriccion, UnidadDeTrabajo,
    };
    use limen_dominio::cedula::Cedula;
    use limen_dominio::contratista::{Contratista, ContratistaGuardado, ContratistaId};
    use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
    use limen_dominio::nombre::NombrePersona;
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales};
    use uuid::Uuid;

    fn contratista(id: u128, cedula: &str) -> Contratista {
        Contratista::restaurar(ContratistaGuardado {
            id: ContratistaId::desde_uuid(Uuid::from_u128(id)),
            cedula: Cedula::normalizar(cedula).unwrap(),
            nombre: NombrePersona::nuevo("ANA").unwrap(),
            empresa: EmpresaId::desde_uuid(Uuid::from_u128(100)),
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            tiene_acceso: true,
        })
    }

    fn empresa(id: u128, nombre: &str) -> Empresa {
        Empresa::restaurar(
            EmpresaId::desde_uuid(Uuid::from_u128(id)),
            NombreEmpresa::nuevo(nombre).unwrap(),
        )
    }

    #[tokio::test]
    async fn sin_confirmar_no_se_escribe_nada() {
        let almacen = AlmacenMemoria::new();
        let mut uow = almacen.nueva();
        uow.contratistas().guardar(&contratista(1, "111111111"));
        drop(uow);
        assert!(
            almacen.contratistas().is_empty(),
            "soltar la UoW es cancelar"
        );
        assert_eq!(almacen.confirmaciones(), 0);
    }

    #[tokio::test]
    async fn confirmar_aplica_lo_anotado() {
        let almacen = AlmacenMemoria::new();
        let mut uow = almacen.nueva();
        uow.contratistas().guardar(&contratista(1, "111111111"));
        uow.empresas().guardar(&empresa(2, "ACME"));
        uow.confirmar().await.unwrap();
        assert_eq!(almacen.contratistas().len(), 1);
        assert_eq!(almacen.empresas().len(), 1);
        assert_eq!(almacen.confirmaciones(), 1);
    }

    #[tokio::test]
    async fn una_lectura_no_ve_lo_anotado_en_la_misma_uow() {
        let almacen = AlmacenMemoria::new();
        let mut uow = almacen.nueva();
        let nuevo = contratista(1, "111111111");
        uow.contratistas().guardar(&nuevo);
        let visto = uow.contratistas().obtener(nuevo.id()).await.unwrap();
        assert_eq!(visto, None, "igual que la base: sólo se ve lo confirmado");
    }

    #[tokio::test]
    async fn cedula_repetida_entre_contratistas_distintos_choca_al_confirmar() {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_contratista(contratista(1, "111111111"));
        let mut uow = almacen.nueva();
        uow.contratistas().guardar(&contratista(2, "111111111"));
        uow.empresas().guardar(&empresa(3, "ACME"));
        assert_eq!(
            uow.confirmar().await,
            Err(ErrorPersistencia::Conflicto(Restriccion::CedulaContratista))
        );
        assert_eq!(
            almacen.contratistas().len(),
            1,
            "no se aplicó el contratista"
        );
        assert!(almacen.empresas().is_empty(), "ni lo demás: todo o nada");
    }

    #[tokio::test]
    async fn dos_pendientes_con_la_misma_cedula_chocan() {
        let almacen = AlmacenMemoria::new();
        let mut uow = almacen.nueva();
        uow.contratistas().guardar(&contratista(1, "111111111"));
        uow.contratistas().guardar(&contratista(2, "111111111"));
        assert_eq!(
            uow.confirmar().await,
            Err(ErrorPersistencia::Conflicto(Restriccion::CedulaContratista))
        );
    }

    #[tokio::test]
    async fn actualizar_al_mismo_contratista_no_choca_con_su_propia_cedula() {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_contratista(contratista(1, "111111111"));
        let mut uow = almacen.nueva();
        uow.contratistas().guardar(&contratista(1, "111111111"));
        assert_eq!(uow.confirmar().await, Ok(()));
    }

    #[tokio::test]
    async fn dos_contratistas_pueden_intercambiar_cedulas_en_la_misma_uow() {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_contratista(contratista(1, "111111111"));
        almacen.sembrar_contratista(contratista(2, "222222222"));
        let mut uow = almacen.nueva();
        uow.contratistas().guardar(&contratista(1, "222222222"));
        uow.contratistas().guardar(&contratista(2, "111111111"));
        assert_eq!(uow.confirmar().await, Ok(()), "el estado final es válido");
    }

    #[tokio::test]
    async fn nombre_de_empresa_repetido_choca_al_confirmar() {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_empresa(empresa(1, "ACME"));
        let mut uow = almacen.nueva();
        uow.empresas().guardar(&empresa(2, "ACME"));
        assert_eq!(
            uow.confirmar().await,
            Err(ErrorPersistencia::Conflicto(Restriccion::NombreEmpresa))
        );
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

    #[tokio::test]
    async fn cedula_en_uso_ignora_al_propio_contratista() {
        let almacen = AlmacenMemoria::new();
        let existente = contratista(1, "111111111");
        almacen.sembrar_contratista(existente.clone());
        let mut uow = almacen.nueva();
        let cedula = existente.cedula().clone();
        let repo = uow.contratistas();
        assert!(
            repo.cedula_en_uso(&cedula, None).await.unwrap(),
            "la usa otro"
        );
        assert!(
            !repo
                .cedula_en_uso(&cedula, Some(existente.id()))
                .await
                .unwrap(),
            "su propia cédula no cuenta"
        );
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
