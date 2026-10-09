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
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::acceso::ResultadoAcceso;
    use limen_dominio::contratista::ErrorContratista;
    use limen_dominio::empresa::EmpresaId;
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
                Err(error) if error.to_string().contains("locked") => {
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

        let historial = almacen.auditoria_de(id.uuid()).await.unwrap();
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
}
