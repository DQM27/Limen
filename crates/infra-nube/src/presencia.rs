//! Presencia de los equipos: quién está conectado ahora, para el panel de
//! dispositivos.
//!
//! Un solo registro por equipo (`conexion:⟨equipo⟩`) que siempre se
//! reescribe: no crece, no tiene historial y **no entra al registro de
//! cambios**, así que no despierta a los demás equipos. Se escribe al
//! conectar (el panel lo ve en vivo) y, como red de seguridad por si el
//! equipo se cae sin avisar, cada [`LATIDO`] mientras la app esté abierta. La
//! hora es la de la nube.

use std::time::Duration;

use limen_dominio::origen::Origen;

use crate::{ErrorNube, Nube};

/// Cada cuánto avisa un equipo conectado que sigue vivo. El panel da por
/// caído al que lleve más de dos de estos sin avisar.
pub const LATIDO: Duration = Duration::from_secs(5 * 60);

impl Nube {
    /// El equipo acaba de conectarse: queda en línea y se anota el inicio.
    pub async fn anunciar_conexion(&self, origen: &Origen) -> Result<(), ErrorNube> {
        self.escribir_presencia(
            origen,
            "{ unidad: $unidad, equipo: $equipo, en_linea: true, \
             iniciada: time::now(), visto: time::now() }",
        )
        .await
    }

    /// El equipo sigue vivo: sólo cambia la hora en que se le vio.
    pub async fn latir(&self, origen: &Origen) -> Result<(), ErrorNube> {
        self.escribir_presencia(
            origen,
            "{ unidad: $unidad, equipo: $equipo, en_linea: true, visto: time::now() }",
        )
        .await
    }

    /// El equipo se despide (la app se cierra de forma normal).
    pub async fn anunciar_salida(&self, origen: &Origen) -> Result<(), ErrorNube> {
        self.escribir_presencia(
            origen,
            "{ unidad: $unidad, equipo: $equipo, en_linea: false, visto: time::now() }",
        )
        .await
    }

    async fn escribir_presencia(&self, origen: &Origen, campos: &str) -> Result<(), ErrorNube> {
        // `campos` es siempre una de las tres constantes de arriba: los
        // datos del equipo van como parámetros.
        let consulta = format!("UPSERT type::record('conexion', $equipo) MERGE {campos}");
        self.db
            .query(consulta)
            .bind(("equipo", origen.equipo.to_string()))
            .bind(("unidad", origen.unidad.as_str().to_owned()))
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map(drop)
            .map_err(|e| ErrorNube::Consulta(e.to_string()))
    }
}

#[cfg(test)]
mod pruebas {
    use limen_dominio::origen::{EquipoId, UnidadOperativaId};
    use uuid::Uuid;

    use super::*;
    use crate::ConfigNube;

    #[tokio::test]
    async fn la_presencia_es_un_solo_registro_por_equipo() {
        let nube = Nube::conectar(&ConfigNube {
            url: "mem://".to_owned(),
            credenciales: None,
        })
        .await
        .unwrap();
        nube.aplicar_esquema().await.unwrap();
        let origen = Origen {
            unidad: UnidadOperativaId::nueva("BRISAS").unwrap(),
            equipo: EquipoId::desde_uuid(Uuid::from_u128(7)),
        };

        nube.anunciar_conexion(&origen).await.unwrap();
        nube.latir(&origen).await.unwrap();
        let mut respuesta = nube
            .db
            .query("SELECT VALUE en_linea FROM conexion")
            .await
            .unwrap();
        let en_linea: Vec<bool> = respuesta.take(0).unwrap();
        assert_eq!(en_linea, vec![true], "un registro, en línea");

        nube.anunciar_salida(&origen).await.unwrap();
        let mut respuesta = nube
            .db
            .query("SELECT VALUE en_linea FROM conexion")
            .await
            .unwrap();
        let en_linea: Vec<bool> = respuesta.take(0).unwrap();
        assert_eq!(en_linea, vec![false], "sigue siendo uno, ahora fuera");
    }
}
