//! Lo que se comprobó en `SurrealDB` (base en memoria) antes de aislar las unidades operativas
//! desde la base. Ver `docs/nube.md`, sección 13.
//!
//! - Un equipo con un acceso limitado a su unidad sólo ve y escribe lo de su unidad.
//! - Una persona que ya está adentro en otra unidad se sigue rechazando (regla A7), aunque el
//!   equipo no pueda verla: la clave natural de la presencia no lleva la unidad.
//! - `SHOW CHANGES` (el registro de cambios que usa la sincronización) **está prohibido** para
//!   ese acceso limitado.
//! - Por eso el diario de cambios sería una tabla propia (`cambio`), que la base llena sola con
//!   un evento en cada tabla compartida, y que el equipo lee con sus permisos de unidad.

#[cfg(test)]
mod tests {
    use limen_infra_surreal::ESQUEMA;
    use surrealdb::Surreal;
    use surrealdb::engine::local::{Db, Mem};
    use surrealdb::opt::auth::Record;
    use surrealdb::types::{SurrealValue, Value};

    #[derive(SurrealValue)]
    struct Parametros {
        id: String,
    }

    const ACCESO: &str = "
    DEFINE TABLE OVERWRITE equipo SCHEMAFULL;
    DEFINE FIELD OVERWRITE unidad ON equipo TYPE string;
    DEFINE ACCESS OVERWRITE equipo ON DATABASE TYPE RECORD
        SIGNIN (SELECT * FROM equipo WHERE id = type::record('equipo', $id))
        DURATION FOR SESSION 1h;
    CREATE equipo:brisas SET unidad = 'BRISAS';
    CREATE equipo:cartago SET unidad = 'CARTAGO';
";

    /// Una base con el esquema y el acceso por equipo, y un equipo de BRISAS con la sesión abierta.
    async fn base_con_sesion_de_brisas(definiciones: &str) -> Surreal<Db> {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("limen").use_db("principal").await.unwrap();
        db.query(ESQUEMA).await.unwrap().check().unwrap();
        db.query(ACCESO).await.unwrap().check().unwrap();
        db.query(definiciones).await.unwrap().check().unwrap();
        db.signin(Record {
            namespace: "limen".to_owned(),
            database: "principal".to_owned(),
            access: "equipo".to_owned(),
            params: Parametros {
                id: "brisas".to_owned(),
            },
        })
        .await
        .unwrap();
        db
    }

    async fn filas(db: &Surreal<Db>, consulta: &str) -> Vec<Value> {
        db.query(consulta)
            .await
            .unwrap()
            .take::<Vec<Value>>(0)
            .unwrap()
    }

    const PRESENCIA_POR_UNIDAD: &str = "
    DEFINE TABLE OVERWRITE presencia SCHEMAFULL CHANGEFEED 30d
        PERMISSIONS FOR select, create, update, delete WHERE unidad = $auth.unidad;
    CREATE presencia:`111` SET unidad = 'CARTAGO', via = 'CONTRATISTA', desde = time::now();
    CREATE presencia:`999` SET unidad = 'BRISAS', via = 'CONTRATISTA', desde = time::now();
";

    #[tokio::test]
    async fn un_equipo_con_acceso_limitado_solo_ve_y_escribe_lo_de_su_unidad() {
        let db = base_con_sesion_de_brisas(PRESENCIA_POR_UNIDAD).await;

        assert_eq!(
            filas(&db, "SELECT id FROM presencia").await.len(),
            1,
            "de las dos presencias sólo ve la de su unidad"
        );
        assert_eq!(
        filas(
            &db,
            "CREATE presencia:`222` SET unidad = 'BRISAS', via = 'CONTRATISTA', desde = time::now()"
        )
        .await
        .len(),
        1,
        "puede registrar a una persona nueva en su unidad"
    );
        assert!(
        filas(
            &db,
            "CREATE presencia:`333` SET unidad = 'CARTAGO', via = 'CONTRATISTA', desde = time::now()"
        )
        .await
        .is_empty(),
        "escribir a nombre de otra unidad se rechaza"
    );
    }

    #[tokio::test]
    async fn una_persona_adentro_en_otra_unidad_se_sigue_rechazando_aunque_no_se_vea() {
        let db = base_con_sesion_de_brisas(PRESENCIA_POR_UNIDAD).await;

        let intento = db
        .query("CREATE presencia:`111` SET unidad = 'BRISAS', via = 'CONTRATISTA', desde = time::now()")
        .await
        .unwrap()
        .take::<Vec<Value>>(0);

        let error = intento.expect_err("la cédula 111 ya está adentro en CARTAGO");
        assert!(
            error.to_string().contains("already exists"),
            "el rechazo es el de la clave natural: {error}"
        );
    }

    #[tokio::test]
    async fn el_registro_de_cambios_de_surrealdb_esta_prohibido_para_el_acceso_limitado() {
        let db = base_con_sesion_de_brisas(PRESENCIA_POR_UNIDAD).await;

        for consulta in [
            "SHOW CHANGES FOR TABLE presencia SINCE 0 LIMIT 10",
            "SHOW CHANGES FOR DATABASE SINCE 0 LIMIT 10",
        ] {
            let error = db
                .query(consulta)
                .await
                .unwrap()
                .take::<Vec<Value>>(0)
                .expect_err("no debería poder leer el registro de cambios");
            assert!(
                error.to_string().contains("permissions"),
                "{consulta}: {error}"
            );
        }
    }

    #[tokio::test]
    async fn un_diario_propio_lo_llena_la_base_y_cada_equipo_lee_solo_el_de_su_unidad() {
        let db = base_con_sesion_de_brisas(
            "
        DEFINE TABLE OVERWRITE cambio SCHEMAFULL
            PERMISSIONS FOR select WHERE unidad = $auth.unidad OR unidad = ''
            FOR create, update, delete NONE;
        DEFINE FIELD OVERWRITE unidad ON cambio TYPE string;
        DEFINE FIELD OVERWRITE tabla ON cambio TYPE string;
        DEFINE FIELD OVERWRITE registro ON cambio TYPE record;
        DEFINE FIELD OVERWRITE tipo ON cambio TYPE string;
        DEFINE FIELD OVERWRITE en ON cambio TYPE datetime;
        DEFINE TABLE OVERWRITE presencia SCHEMAFULL
            PERMISSIONS FOR select, create, update, delete WHERE unidad = $auth.unidad;
        DEFINE EVENT OVERWRITE anotar ON TABLE presencia THEN {
            CREATE cambio SET unidad = ($after.unidad ?? $before.unidad), tabla = 'presencia',
                registro = $value.id ?? $before.id, tipo = $event, en = time::now();
        };
        CREATE presencia:`111` SET unidad = 'CARTAGO', via = 'CONTRATISTA', desde = time::now();
        ",
        )
        .await;

        filas(
        &db,
        "CREATE presencia:`222` SET unidad = 'BRISAS', via = 'CONTRATISTA', desde = time::now()",
    )
    .await;
        filas(&db, "DELETE presencia:`222`").await;

        let diario = filas(&db, "SELECT tipo FROM cambio ORDER BY en").await;
        assert_eq!(
            diario.len(),
            2,
            "alta y borrado quedan anotados; el alta de CARTAGO (de antes) no es suya"
        );
        assert!(
        filas(
            &db,
            "CREATE cambio SET unidad = 'BRISAS', tabla = 'x', registro = presencia:`1`, tipo = 'X', en = time::now()"
        )
        .await
        .is_empty(),
        "el equipo no puede escribir en el diario: lo llena sólo la base"
    );
    }
}
