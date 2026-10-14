//! Lo que llega de la nube a la base del equipo, y hasta dónde bajó cada cosa. Ver
//! `docs/nube.md`.
//!
//! Lo que **sube** no se lee de aquí: la bandeja de salida (`bandeja.rs`) lleva la cuenta de lo
//! que hay que mandar. Lo que baja se escribe con `$de_la_nube`, y por eso nunca entra a esa
//! bandeja.

use limen_aplicacion::puertos::ErrorPersistencia;
use surrealdb::types::{RecordId, Value};

use crate::almacen::AlmacenSurreal;
use crate::error::tecnica;

/// Un cambio de una fila.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cambio {
    /// La fila quedó así (alta o cambio): la fila completa, con su `id`.
    Guardada(Value),
    /// La fila se borró.
    Borrada(RecordId),
}

impl AlmacenSurreal {
    /// Hasta qué entrada del registro de `tabla` **en la nube** bajó este
    /// equipo, si alguna vez bajó.
    pub async fn bajado_hasta(&self, tabla: &str) -> Result<Option<i64>, ErrorPersistencia> {
        self.cursor(format!("bajada-{tabla}")).await
    }

    /// Anota que el registro de `tabla` en la nube ya bajó hasta `numero`.
    pub async fn anotar_bajado_hasta(
        &self,
        tabla: &str,
        numero: i64,
    ) -> Result<(), ErrorPersistencia> {
        self.anotar_cursor(format!("bajada-{tabla}"), numero).await
    }

    /// Hasta dónde bajó este equipo el diario de cambios de la nube (microsegundos de la última
    /// anotación que leyó), si alguna vez bajó.
    pub async fn bajado_hasta_en_el_diario(&self) -> Result<Option<i64>, ErrorPersistencia> {
        self.cursor("bajada-del-diario".to_owned()).await
    }

    /// Anota hasta dónde bajó el diario de cambios de la nube.
    pub async fn anotar_bajado_hasta_en_el_diario(
        &self,
        micros: i64,
    ) -> Result<(), ErrorPersistencia> {
        self.anotar_cursor("bajada-del-diario".to_owned(), micros)
            .await
    }

    pub(crate) async fn cursor(&self, clave: String) -> Result<Option<i64>, ErrorPersistencia> {
        let mut respuesta = self
            .db()
            .query("SELECT VALUE hasta FROM ONLY type::record('sincronizacion', $clave)")
            .bind(("clave", clave))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    pub(crate) async fn anotar_cursor(
        &self,
        clave: String,
        numero: i64,
    ) -> Result<(), ErrorPersistencia> {
        self.db()
            .query("UPSERT type::record('sincronizacion', $clave) CONTENT { hasta: $numero }")
            .bind(("clave", clave))
            .bind(("numero", numero))
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map(drop)
            .map_err(tecnica)
    }

    /// Aplica en la base del equipo cambios que llegan de la nube, en una
    /// sola transacción, y devuelve cuántas filas cambiaron de verdad.
    ///
    /// **Lo que ya está igual no se escribe**, y lo que sí se escribe va marcado
    /// `$de_la_nube`: los eventos de la bandeja de salida (`esquema/equipo.surql`) lo ven y no
    /// anotan nada, así que lo que llega no vuelve a subir. Con `inmutable` (hechos y
    /// auditoría) sólo se agrega lo que falta: nunca se reemplaza ni se borra nada.
    pub async fn aplicar_de_la_nube(
        &self,
        cambios: Vec<Cambio>,
        inmutable: bool,
    ) -> Result<usize, ErrorPersistencia> {
        let mut sentencias = vec!["BEGIN TRANSACTION;".to_owned()];
        let mut valores: Vec<(String, Value)> = Vec::new();
        for (i, cambio) in cambios.into_iter().enumerate() {
            match cambio {
                Cambio::Guardada(fila) => {
                    let Some(id) = id_de(&fila) else { continue };
                    match self.fila(&id).await? {
                        Some(actual) if inmutable || actual == fila => continue,
                        Some(_) => sentencias.push(format!("UPSERT $c{i}.id CONTENT $c{i};")),
                        None => sentencias.push(format!("CREATE $c{i}.id CONTENT $c{i};")),
                    }
                    valores.push((format!("c{i}"), fila));
                }
                Cambio::Borrada(id) => {
                    if inmutable || self.fila(&id).await?.is_none() {
                        continue;
                    }
                    sentencias.push(format!("DELETE $c{i};"));
                    valores.push((format!("c{i}"), Value::RecordId(id)));
                }
            }
        }
        let escritas = valores.len();
        if escritas == 0 {
            return Ok(0);
        }
        sentencias.push("COMMIT TRANSACTION;".to_owned());
        let mut consulta = self
            .db()
            .query(sentencias.join("\n"))
            .bind(("de_la_nube", true));
        for (nombre, valor) in valores {
            consulta = consulta.bind((nombre, valor));
        }
        consulta
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map_err(tecnica)?;
        Ok(escritas)
    }

    pub(crate) async fn fila(&self, id: &RecordId) -> Result<Option<Value>, ErrorPersistencia> {
        let mut respuesta = self
            .db()
            .query("SELECT * FROM ONLY $id")
            .bind(("id", id.clone()))
            .await
            .map_err(tecnica)?;
        let fila: Value = respuesta.take(0).map_err(tecnica)?;
        Ok(match fila {
            Value::None | Value::Null => None,
            otra => Some(otra),
        })
    }
}

/// De qué tabla es un cambio.
pub fn tabla_de(cambio: &Cambio) -> Option<&str> {
    let id = match cambio {
        Cambio::Guardada(Value::Object(campos)) => match campos.get("id") {
            Some(Value::RecordId(id)) => id,
            _ => return None,
        },
        Cambio::Guardada(_) => return None,
        Cambio::Borrada(id) => id,
    };
    Some(id.table.as_str())
}

/// El `id` de una fila completa.
pub fn id_de(fila: &Value) -> Option<RecordId> {
    match fila {
        Value::Object(campos) => match campos.get("id") {
            Some(Value::RecordId(id)) => Some(id.clone()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La fila de una presencia tal como la manda la nube.
    async fn presencia_de_la_nube(almacen: &AlmacenSurreal) -> Value {
        let mut respuesta = almacen
            .db()
            .query(
                "RETURN { id: presencia:`222222222`, unidad: 'PRUEBAS', via: 'PROVEEDOR', \
                 desde: d'2026-10-09T09:00:00Z' }",
            )
            .await
            .unwrap();
        respuesta.take(0).unwrap()
    }

    async fn por_subir(almacen: &AlmacenSurreal) -> usize {
        almacen
            .pendientes_de_la_bandeja("presencia", 1000)
            .await
            .unwrap()
            .len()
    }

    #[tokio::test]
    async fn recuerda_hasta_donde_bajo_cada_tabla() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        assert_eq!(almacen.bajado_hasta("presencia").await.unwrap(), None);

        almacen.anotar_bajado_hasta("presencia", 42).await.unwrap();
        almacen.anotar_bajado_hasta("presencia", 43).await.unwrap();

        assert_eq!(almacen.bajado_hasta("presencia").await.unwrap(), Some(43));
        assert_eq!(
            almacen.bajado_hasta("gafete").await.unwrap(),
            None,
            "cada tabla el suyo"
        );
    }

    #[tokio::test]
    async fn lo_que_llega_de_la_nube_se_aplica_una_vez_y_no_rebota() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let fila = presencia_de_la_nube(&almacen).await;
        let id = id_de(&fila).unwrap();

        let escritas = almacen
            .aplicar_de_la_nube(vec![Cambio::Guardada(fila.clone())], false)
            .await
            .unwrap();
        assert_eq!(escritas, 1, "la fila nueva se escribe");
        assert_eq!(
            por_subir(&almacen).await,
            0,
            "pero no entra a la bandeja de salida: viene de la nube"
        );

        let otra_vez = almacen
            .aplicar_de_la_nube(vec![Cambio::Guardada(fila)], false)
            .await
            .unwrap();
        assert_eq!(otra_vez, 0, "lo que ya está igual no se escribe");

        let borradas = almacen
            .aplicar_de_la_nube(vec![Cambio::Borrada(id.clone())], false)
            .await
            .unwrap();
        assert_eq!(borradas, 1, "el borrado se aplica");
        assert_eq!(por_subir(&almacen).await, 0, "y tampoco sube");
        assert_eq!(
            almacen
                .aplicar_de_la_nube(vec![Cambio::Borrada(id)], false)
                .await
                .unwrap(),
            0,
            "borrar lo que no está no hace nada"
        );
    }

    #[tokio::test]
    async fn lo_inmutable_solo_se_agrega() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let fila = presencia_de_la_nube(&almacen).await;
        let id = id_de(&fila).unwrap();

        assert_eq!(
            almacen
                .aplicar_de_la_nube(vec![Cambio::Guardada(fila.clone())], true)
                .await
                .unwrap(),
            1,
            "lo que falta se agrega"
        );
        let mut respuesta = almacen
            .db()
            .query("RETURN { id: presencia:`222222222`, unidad: 'PRUEBAS', via: 'KOF', desde: d'2026-10-09T09:00:00Z' }")
            .await
            .unwrap();
        let cambiada: Value = respuesta.take(0).unwrap();
        assert_eq!(
            almacen
                .aplicar_de_la_nube(vec![Cambio::Guardada(cambiada), Cambio::Borrada(id)], true)
                .await
                .unwrap(),
            0,
            "lo que ya está no se reemplaza ni se borra"
        );
    }
}
