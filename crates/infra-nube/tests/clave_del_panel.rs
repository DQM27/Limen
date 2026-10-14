//! El panel crea usuarios y restablece claves en la nube, y cifra la clave temporal con la
//! propia base (`crypto::argon2::generate`). Lo que importa: que el equipo, que verifica con
//! el cifrador de Limen (Argon2id, formato PHC), acepte ese hash. Si algún día la base cambia
//! de formato o de algoritmo, esta prueba lo dice antes de que alguien se quede sin entrar.

#[cfg(test)]
mod tests {
    use limen_aplicacion::puertos::Claves;
    use limen_dominio::usuario::HashClave;
    use limen_infra_plataforma::ClavesArgon2;
    use surrealdb::engine::any::connect;

    const CLAVE_TEMPORAL: &str = "7KQ2MXW9PB";

    async fn hash_de_la_base(clave: &str) -> String {
        let db = connect("mem://").await.unwrap();
        db.use_ns("prueba").use_db("prueba").await.unwrap();
        let mut respuesta = db
            .query("RETURN crypto::argon2::generate($clave);")
            .bind(("clave", clave.to_owned()))
            .await
            .unwrap();
        respuesta.take::<Option<String>>(0).unwrap().unwrap()
    }

    #[tokio::test]
    async fn el_hash_de_la_base_es_argon2id_y_el_equipo_lo_acepta() {
        let hash = hash_de_la_base(CLAVE_TEMPORAL).await;
        assert!(
            hash.starts_with("$argon2id$"),
            "formato PHC Argon2id: {hash}"
        );

        let claves = ClavesArgon2::new().unwrap();
        let guardado = HashClave::desde_texto(hash);
        assert!(
            claves.verificar(CLAVE_TEMPORAL, Some(&guardado)),
            "la clave temporal entra con el hash que hizo la base"
        );
        assert!(
            !claves.verificar("OTRA-CLAVE-99", Some(&guardado)),
            "otra clave no entra"
        );
    }

    #[tokio::test]
    async fn cada_hash_lleva_su_propia_sal() {
        assert_ne!(
            hash_de_la_base(CLAVE_TEMPORAL).await,
            hash_de_la_base(CLAVE_TEMPORAL).await,
            "dos claves iguales no dan el mismo hash"
        );
    }
}
