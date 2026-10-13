//! El registro de cambios de la base del equipo (`CHANGEFEED`): lo que se
//! sube a la nube. Ver `docs/nube.md`.
//!
//! Cada tabla compartida anota sus altas, cambios y borrados durante 30
//! días. El equipo recuerda hasta dónde subió cada tabla en
//! `sincronizacion:⟨tabla⟩` y, al volver la red, sube lo anotado desde ahí.

use limen_aplicacion::puertos::ErrorPersistencia;
use surrealdb::types::{Number, RecordId, Value};

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

/// Los cambios de una transacción, con su lugar en el registro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entrada {
    /// Crece con cada transacción: es el cursor para seguir desde aquí.
    pub numero: i64,
    pub cambios: Vec<Cambio>,
}

impl AlmacenSurreal {
    /// Hasta qué entrada del registro de `tabla` subió este equipo, si
    /// alguna vez subió.
    pub async fn subido_hasta(&self, tabla: &str) -> Result<Option<i64>, ErrorPersistencia> {
        self.cursor(tabla.to_owned()).await
    }

    /// Anota que el registro de `tabla` ya subió hasta la entrada `numero`.
    pub async fn anotar_subido_hasta(
        &self,
        tabla: &str,
        numero: i64,
    ) -> Result<(), ErrorPersistencia> {
        self.anotar_cursor(tabla.to_owned(), numero).await
    }

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

    /// Hasta dónde subió este equipo el registro de cambios **de toda la base**, si alguna
    /// vez subió. Es un solo cursor para todas las tablas: con uno por tabla, una tabla
    /// tranquila (como `hecho`) se quedaba atrás y `SurrealDB`, que corta el escaneo antes de
    /// filtrar por tabla, dejaba de verle las entradas nuevas.
    pub async fn subido_hasta_en_la_base(&self) -> Result<Option<i64>, ErrorPersistencia> {
        self.cursor("subida-de-la-base".to_owned()).await
    }

    /// Anota que el registro de toda la base ya subió hasta la entrada `numero`.
    pub async fn anotar_subido_hasta_en_la_base(
        &self,
        numero: i64,
    ) -> Result<(), ErrorPersistencia> {
        self.anotar_cursor("subida-de-la-base".to_owned(), numero)
            .await
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

    async fn cursor(&self, clave: String) -> Result<Option<i64>, ErrorPersistencia> {
        let mut respuesta = self
            .db()
            .query("SELECT VALUE hasta FROM ONLY type::record('sincronizacion', $clave)")
            .bind(("clave", clave))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    async fn anotar_cursor(&self, clave: String, numero: i64) -> Result<(), ErrorPersistencia> {
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
    /// **Lo que ya está igual no se escribe:** así no se anota en el
    /// registro de cambios del equipo, no vuelve a subir y un cambio no
    /// rebota entre los equipos y la nube. Con `inmutable` (hechos y
    /// auditoría) sólo se agrega lo que falta: nunca se reemplaza ni se
    /// borra nada.
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
        let mut consulta = self.db().query(sentencias.join("\n"));
        for (nombre, valor) in valores {
            consulta = consulta.bind((nombre, valor));
        }
        consulta
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map_err(tecnica)?;
        Ok(escritas)
    }

    async fn fila(&self, id: &RecordId) -> Result<Option<Value>, ErrorPersistencia> {
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

    /// Las entradas del registro de `tabla` desde la número `desde`
    /// (incluida), en orden, a lo sumo `limite`.
    pub async fn cambios_desde(
        &self,
        tabla: &str,
        desde: i64,
        limite: usize,
    ) -> Result<Vec<Entrada>, ErrorPersistencia> {
        // `SINCE` y `LIMIT` sólo aceptan valores literales: son números, y
        // la tabla sale de una lista fija del adaptador, nunca de un dato.
        let mut respuesta = self
            .db()
            .query(format!(
                "SHOW CHANGES FOR TABLE {tabla} SINCE {desde} LIMIT {limite}"
            ))
            .await
            .map_err(tecnica)?;
        let crudo: Value = respuesta.take(0).map_err(tecnica)?;
        entradas(crudo)
    }

    /// Las entradas del registro de **toda la base** desde la número `desde` (incluida), en
    /// orden, a lo sumo `limite`. Trae las de todas las tablas con registro de cambios, así
    /// que el cursor siempre avanza; quien llama reparte los cambios por tabla.
    pub async fn cambios_de_la_base_desde(
        &self,
        desde: i64,
        limite: usize,
    ) -> Result<Vec<Entrada>, ErrorPersistencia> {
        // `SINCE` y `LIMIT` sólo aceptan valores literales: son números.
        let mut respuesta = self
            .db()
            .query(format!(
                "SHOW CHANGES FOR DATABASE SINCE {desde} LIMIT {limite}"
            ))
            .await
            .map_err(tecnica)?;
        let crudo: Value = respuesta.take(0).map_err(tecnica)?;
        entradas(crudo)
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

/// Lee lo que devuelve `SHOW CHANGES` (de la base del equipo o de la nube,
/// que tienen el mismo esquema).
pub fn entradas(crudo: Value) -> Result<Vec<Entrada>, ErrorPersistencia> {
    let Value::Array(lista) = crudo else {
        return Err(raro("el registro de cambios no es una lista"));
    };
    lista.into_inner().into_iter().map(entrada).collect()
}

fn entrada(valor: Value) -> Result<Entrada, ErrorPersistencia> {
    let Value::Object(campos) = valor else {
        return Err(raro("una entrada del registro no es un objeto"));
    };
    let mut campos = campos.into_inner();
    let Some(Value::Number(Number::Int(numero))) = campos.remove("versionstamp") else {
        return Err(raro("una entrada del registro sin su número"));
    };
    let cambios = match campos.remove("changes") {
        Some(Value::Array(cambios)) => cambios.into_inner(),
        _ => Vec::new(),
    };
    Ok(Entrada {
        numero,
        cambios: cambios.into_iter().filter_map(cambio).collect(),
    })
}

/// Un cambio de una fila; `None` para lo que no es de filas (por ejemplo,
/// la definición de la tabla al aplicar el esquema).
fn cambio(valor: Value) -> Option<Cambio> {
    let Value::Object(campos) = valor else {
        return None;
    };
    let mut campos = campos.into_inner();
    if let Some(fila @ Value::Object(_)) = campos.remove("update") {
        return Some(Cambio::Guardada(fila));
    }
    match campos.remove("delete") {
        Some(Value::Object(borrada)) => match borrada.into_inner().remove("id") {
            Some(Value::RecordId(id)) => Some(Cambio::Borrada(id)),
            _ => None,
        },
        _ => None,
    }
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

fn raro(detalle: &str) -> ErrorPersistencia {
    ErrorPersistencia::Tecnica(format!("registro de cambios: {detalle}"))
}

#[cfg(test)]
mod tests {
    use limen_aplicacion::puertos::{
        FabricaUnidadDeTrabajo, RepositorioPresencias, UnidadDeTrabajo,
    };
    use limen_dominio::cedula::Cedula;
    use limen_dominio::presencia::{Identidad, Via};

    use super::*;

    #[tokio::test]
    async fn anota_las_altas_y_los_borrados_en_orden() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let identidad = Identidad::from(&Cedula::normalizar("111111111").unwrap());
        let en = "2026-10-09T08:00:00Z".parse().unwrap();
        let mut uow = almacen.nueva();
        uow.presencias()
            .anotar_entrada(&identidad, Via::Contratista, en);
        uow.confirmar().await.unwrap();
        let mut uow = almacen.nueva();
        uow.presencias().anotar_salida(&identidad);
        uow.confirmar().await.unwrap();

        let entradas = almacen.cambios_desde("presencia", 0, 100).await.unwrap();
        let cambios: Vec<&Cambio> = entradas.iter().flat_map(|e| &e.cambios).collect();

        assert!(
            matches!(
                cambios.as_slice(),
                [Cambio::Guardada(_), Cambio::Borrada(_)]
            ),
            "el alta y después el borrado: {cambios:?}"
        );
        assert!(
            entradas.windows(2).all(|par| par[0].numero < par[1].numero),
            "en orden"
        );
        let ultima = entradas.last().map(|e| e.numero).unwrap();
        assert!(
            almacen
                .cambios_desde("presencia", ultima + 1, 100)
                .await
                .unwrap()
                .is_empty(),
            "después de la última no hay nada"
        );
    }

    #[tokio::test]
    async fn recuerda_hasta_donde_subio_cada_tabla() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        assert_eq!(almacen.subido_hasta("presencia").await.unwrap(), None);

        almacen.anotar_subido_hasta("presencia", 42).await.unwrap();
        almacen.anotar_subido_hasta("presencia", 43).await.unwrap();

        assert_eq!(almacen.subido_hasta("presencia").await.unwrap(), Some(43));
        assert_eq!(
            almacen.subido_hasta("gafete").await.unwrap(),
            None,
            "cada tabla el suyo"
        );
    }

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

    async fn anotados(almacen: &AlmacenSurreal) -> usize {
        almacen
            .cambios_desde("presencia", 0, 1000)
            .await
            .unwrap()
            .iter()
            .map(|entrada| entrada.cambios.len())
            .sum()
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
        let anotados_tras_llegar = anotados(&almacen).await;

        let otra_vez = almacen
            .aplicar_de_la_nube(vec![Cambio::Guardada(fila)], false)
            .await
            .unwrap();
        assert_eq!(otra_vez, 0, "lo que ya está igual no se escribe");
        assert_eq!(
            anotados(&almacen).await,
            anotados_tras_llegar,
            "ni se anota: no vuelve a subir"
        );

        let borradas = almacen
            .aplicar_de_la_nube(vec![Cambio::Borrada(id.clone())], false)
            .await
            .unwrap();
        assert_eq!(borradas, 1, "el borrado se aplica");
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
                .aplicar_de_la_nube(vec![Cambio::Guardada(fila)], true)
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
