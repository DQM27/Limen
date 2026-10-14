//! El acceso del panel a la nube, de sólo lectura y en tiempo real. Ver `docs/nube.md`, sección 14.
//!
//! La página del panel se conecta directo a la base con un token de una hora que le da el Worker
//! (`DEFINE ACCESS panel ... TYPE RECORD WITH JWT`, que define aparte el dueño porque lleva la clave
//! de firma). Lo que puede hacer lo decide la base, no la página:
//!
//! - ve los hechos de **todas** las unidades, los equipos y su conexión, y los usuarios;
//! - **nunca** ve la clave (el hash) de un usuario;
//! - no escribe nada: ni hechos, ni catálogos, ni usuarios, ni conexión.
//!
//! Y lo que ya hacían los equipos se conserva: ven sólo lo de su unidad, leen la clave de los
//! usuarios (la necesitan para iniciar sesión) y siguen creando y editando los catálogos globales.

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use hmac::{Hmac, Mac};
    use limen_infra_surreal::ESQUEMA;
    use sha2::Sha512;
    use surrealdb::Surreal;
    use surrealdb::engine::local::{Db, Mem};
    use surrealdb::types::Value;

    const ESQUEMA_NUBE: &str = include_str!("../src/esquema_nube.surql");
    const CLAVE_EQUIPOS: &str = "clave-de-los-equipos-que-solo-conocen-el-worker-y-la-base";
    const CLAVE_PANEL: &str = "clave-del-panel-que-solo-conocen-el-worker-y-la-base-0123456789";

    /// Los dos accesos, como los define el dueño en la nube. El del panel no lleva la unidad de
    /// nadie: lo único que importa es que su sesión se llama `panel`.
    const ACCESOS: &str = "
        DEFINE ACCESS OVERWRITE equipo ON DATABASE TYPE RECORD WITH JWT ALGORITHM HS512
            KEY 'clave-de-los-equipos-que-solo-conocen-el-worker-y-la-base'
            AUTHENTICATE { RETURN type::record('equipo', $token.equipo) }
            DURATION FOR SESSION 1h;
        DEFINE ACCESS OVERWRITE panel ON DATABASE TYPE RECORD WITH JWT ALGORITHM HS512
            KEY 'clave-del-panel-que-solo-conocen-el-worker-y-la-base-0123456789'
            AUTHENTICATE { RETURN type::record('panel_lector', 'panel') }
            DURATION FOR SESSION 1h;
    ";

    /// Lo que hay en la nube, puesto por la administración: hechos de dos unidades, dos equipos con
    /// su conexión, un usuario con su clave y un catálogo.
    const DATOS: &str = "
        CREATE hecho SET tipo = 'ENTRADA', via = 'CONTRATISTA', registro = rand::uuid(),
            en = time::now(), operador = rand::uuid(), unidad = 'BRISAS', equipo = rand::uuid(),
            hora_confiable = true, hora_equipo = time::now();
        CREATE hecho SET tipo = 'ENTRADA', via = 'PROVEEDOR', registro = rand::uuid(),
            en = time::now(), operador = rand::uuid(), unidad = 'CARTAGO', equipo = rand::uuid(),
            hora_confiable = true, hora_equipo = time::now();
        CREATE conexion:a1 SET unidad = 'BRISAS', equipo = 'a1', en_linea = true, visto = time::now();
        CREATE conexion:b1 SET unidad = 'CARTAGO', equipo = 'b1', en_linea = false, visto = time::now();
        CREATE dispositivo:a1 SET nombre = 'A1', unidad = 'BRISAS', tipo = 'pc', creado = time::now();
        CREATE usuario:ana SET cedula = '111111111', nombre = 'ANA MORA', activo = true,
            clave = '$argon2id$v=19$m=19456,t=2,p=1$sal$hash', debe_cambiar_clave = false;
        CREATE empresa:acme SET nombre = 'ACME', nombre_busqueda = 'acme';
    ";

    fn token(acceso: &str, clave: &str, extra: &str) -> String {
        let cabecera = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS512","typ":"JWT"}"#);
        let datos = format!(
            r#"{{"ns":"limen","db":"principal","ac":"{acceso}"{extra},"iat":1700000000,"exp":4000000000}}"#
        );
        let cuerpo = format!("{cabecera}.{}", URL_SAFE_NO_PAD.encode(datos));
        let mut firma = Hmac::<Sha512>::new_from_slice(clave.as_bytes()).unwrap();
        firma.update(cuerpo.as_bytes());
        format!(
            "{cuerpo}.{}",
            URL_SAFE_NO_PAD.encode(firma.finalize().into_bytes())
        )
    }

    /// Una base con el esquema de la nube y los datos, y la sesión ya abierta con `token`.
    async fn base(token: &str) -> Surreal<Db> {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("limen").use_db("principal").await.unwrap();
        db.query(ESQUEMA).await.unwrap().check().unwrap();
        db.query(ESQUEMA_NUBE).await.unwrap().check().unwrap();
        db.query(ACCESOS).await.unwrap().check().unwrap();
        db.query(DATOS).await.unwrap().check().unwrap();
        db.authenticate(token).await.unwrap();
        db
    }

    async fn panel() -> Surreal<Db> {
        base(&token("panel", CLAVE_PANEL, "")).await
    }

    async fn equipo_de_brisas() -> Surreal<Db> {
        base(&token(
            "equipo",
            CLAVE_EQUIPOS,
            r#","unidad":"BRISAS","equipo":"a1""#,
        ))
        .await
    }

    async fn filas(db: &Surreal<Db>, consulta: &str) -> Vec<Value> {
        db.query(consulta)
            .await
            .unwrap()
            .take::<Vec<Value>>(0)
            .unwrap()
    }

    #[tokio::test]
    async fn el_panel_ve_los_hechos_de_todas_las_unidades_y_los_equipos() {
        let db = panel().await;
        assert_eq!(
            filas(&db, "SELECT unidad FROM hecho").await.len(),
            2,
            "los hechos de BRISAS y de CARTAGO"
        );
        assert_eq!(filas(&db, "SELECT id FROM conexion").await.len(), 2);
        assert_eq!(filas(&db, "SELECT id FROM dispositivo").await.len(), 1);
        assert_eq!(filas(&db, "SELECT id FROM usuario").await.len(), 1);
    }

    #[tokio::test]
    async fn el_panel_nunca_ve_la_clave_de_un_usuario() {
        let db = panel().await;
        let usuarios = filas(&db, "SELECT * FROM usuario").await;
        assert_eq!(usuarios.len(), 1);
        let texto = format!("{usuarios:?}");
        assert!(texto.contains("ANA MORA"), "ve al usuario: {texto}");
        assert!(
            !texto.contains("argon2id") && !texto.contains("\"clave\""),
            "pero no su clave: {texto}"
        );
    }

    #[tokio::test]
    async fn el_panel_no_escribe_nada() {
        let db = panel().await;
        for escritura in [
            "UPDATE usuario:ana SET activo = false",
            "CREATE usuario:luis SET cedula = '222222222', nombre = 'LUIS', activo = true, clave = 'x', debe_cambiar_clave = false",
            "DELETE usuario:ana",
            "UPDATE conexion:a1 SET en_linea = false",
            "CREATE conexion:c1 SET unidad = 'BRISAS', equipo = 'c1', en_linea = true, visto = time::now()",
            "UPDATE dispositivo:a1 SET retirado = time::now()",
            "UPDATE empresa:acme SET nombre = 'OTRA'",
            "CREATE empresa:otra SET nombre = 'OTRA', nombre_busqueda = 'otra'",
            "CREATE hecho SET tipo = 'ENTRADA', via = 'CONTRATISTA', registro = rand::uuid(), en = time::now(), operador = rand::uuid(), unidad = 'BRISAS', equipo = rand::uuid(), hora_confiable = true, hora_equipo = time::now()",
            "CREATE auditoria SET entidad = 'usuario', registro = 'x', accion = 'alta', cambios = [], operador = rand::uuid(), en = time::now()",
        ] {
            assert!(
                filas(&db, escritura).await.is_empty(),
                "el panel no debe poder: {escritura}"
            );
        }
        // Y nada cambió.
        let usuarios = filas(&db, "SELECT activo FROM usuario").await;
        assert!(format!("{usuarios:?}").contains("true"), "{usuarios:?}");
        assert_eq!(filas(&db, "SELECT id FROM hecho").await.len(), 2);
    }

    #[tokio::test]
    async fn un_equipo_sigue_viendo_solo_lo_de_su_unidad() {
        let db = equipo_de_brisas().await;
        let hechos = filas(&db, "SELECT unidad FROM hecho").await;
        assert_eq!(hechos.len(), 1, "sólo el de BRISAS: {hechos:?}");
        assert!(
            filas(&db, "SELECT id FROM conexion").await.is_empty(),
            "la conexión de los equipos es sólo del panel"
        );
        assert_eq!(
            filas(&db, "SELECT id FROM dispositivo").await.len(),
            0,
            "los dispositivos también son sólo del panel"
        );
    }

    #[tokio::test]
    async fn un_equipo_sigue_leyendo_la_clave_de_los_usuarios_para_iniciar_sesion() {
        let db = equipo_de_brisas().await;
        let usuarios = filas(&db, "SELECT clave FROM usuario").await;
        assert!(
            format!("{usuarios:?}").contains("argon2id"),
            "sin la clave nadie podría iniciar sesión en el equipo: {usuarios:?}"
        );
    }

    #[tokio::test]
    async fn un_equipo_sigue_creando_y_editando_los_catalogos_globales() {
        let db = equipo_de_brisas().await;
        assert_eq!(
            filas(
                &db,
                "CREATE empresa:nueva SET nombre = 'NUEVA', nombre_busqueda = 'nueva'"
            )
            .await
            .len(),
            1,
            "puede agregar una empresa"
        );
        assert_eq!(
            filas(&db, "UPDATE empresa:acme SET nombre = 'ACME 2'")
                .await
                .len(),
            1,
            "y editarla"
        );
        assert!(
            filas(&db, "DELETE empresa:acme").await.is_empty(),
            "pero no borrarla"
        );
        assert!(
            filas(&db, "UPDATE usuario:ana SET activo = false")
                .await
                .is_empty(),
            "ni tocar a los usuarios: los administra la nube"
        );
    }
}
