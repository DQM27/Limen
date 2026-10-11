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
        let mut respuesta = self
            .db()
            .query("SELECT VALUE hasta FROM ONLY type::record('sincronizacion', $tabla)")
            .bind(("tabla", tabla.to_owned()))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    /// Anota que el registro de `tabla` ya subió hasta la entrada `numero`.
    pub async fn anotar_subido_hasta(
        &self,
        tabla: &str,
        numero: i64,
    ) -> Result<(), ErrorPersistencia> {
        self.db()
            .query("UPSERT type::record('sincronizacion', $tabla) CONTENT { hasta: $numero }")
            .bind(("tabla", tabla.to_owned()))
            .bind(("numero", numero))
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map(drop)
            .map_err(tecnica)
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
}

fn entradas(crudo: Value) -> Result<Vec<Entrada>, ErrorPersistencia> {
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
}
