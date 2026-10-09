//! Conexión a la base embebida y aplicación del esquema.

use std::path::Path;

use limen_aplicacion::puertos::ErrorPersistencia;
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, SurrealKv};

use uuid::Uuid;

use crate::error::tecnica;
use crate::registros::AuditoriaRegistro;

const ESQUEMA: &str = include_str!("esquema/esquema.surql");
const NAMESPACE: &str = "limen";
const BASE: &str = "principal";

/// La base de datos embebida. Clonarla comparte la misma conexión: es lo
/// que reciben la composición y la fábrica de Unit of Work.
#[derive(Debug, Clone)]
pub struct AlmacenSurreal {
    db: Surreal<Db>,
}

impl AlmacenSurreal {
    /// Base en memoria, nueva y vacía, con el esquema aplicado. Para pruebas.
    pub async fn en_memoria() -> Result<Self, ErrorPersistencia> {
        let db = Surreal::new::<Mem>(()).await.map_err(tecnica)?;
        Self::preparar(db).await
    }

    /// Base en disco (`SurrealKV`) en `ruta`, con el esquema aplicado. Si no
    /// existe, se crea.
    ///
    /// Una sola apertura por proceso: el archivo queda tomado mientras viva
    /// algún clon del almacén, y al soltar el último el motor lo libera en
    /// segundo plano (no hay un cierre que se pueda esperar). Abrir la misma
    /// ruta dos veces a la vez falla con un error técnico.
    pub async fn en_disco(ruta: &Path) -> Result<Self, ErrorPersistencia> {
        let ruta = ruta.to_str().ok_or_else(|| {
            ErrorPersistencia::Tecnica(format!("ruta no válida: {}", ruta.display()))
        })?;
        let db = Surreal::new::<SurrealKv>(ruta).await.map_err(tecnica)?;
        Self::preparar(db).await
    }

    async fn preparar(db: Surreal<Db>) -> Result<Self, ErrorPersistencia> {
        db.use_ns(NAMESPACE).use_db(BASE).await.map_err(tecnica)?;
        db.query(ESQUEMA)
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map_err(tecnica)?;
        Ok(Self { db })
    }

    /// Historial de auditoría de un registro (contratista o empresa), del
    /// más antiguo al más reciente. Se ordena por el ID de cada entrada
    /// (UUID v7), no por la hora: dos cambios en el mismo instante empatan
    /// en la hora pero no en el ID.
    pub async fn auditoria_de(
        &self,
        registro: Uuid,
    ) -> Result<Vec<AuditoriaRegistro>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * OMIT id FROM auditoria WHERE registro = $registro ORDER BY id")
            .bind(("registro", registro))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    pub(crate) const fn db(&self) -> &Surreal<Db> {
        &self.db
    }
}
