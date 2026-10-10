//! Conexión a la base embebida y aplicación del esquema.

use std::path::Path;

use limen_aplicacion::puertos::{ErrorPersistencia, RegistroAuditado};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, SurrealKv};

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

    /// Historial de auditoría de un registro, del
    /// más antiguo al más reciente. Se ordena por el ID de cada entrada
    /// (UUID v7), no por la hora: dos cambios en el mismo instante empatan
    /// en la hora pero no en el ID.
    pub async fn auditoria_de(
        &self,
        registro: RegistroAuditado,
    ) -> Result<Vec<AuditoriaRegistro>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query(
                "SELECT * OMIT id FROM auditoria \
                 WHERE entidad = $entidad AND registro = $registro ORDER BY id",
            )
            .bind(("entidad", registro.entidad().to_owned()))
            .bind(("registro", registro.clave()))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    pub(crate) const fn db(&self) -> &Surreal<Db> {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use limen_aplicacion::puertos::{
        AccionAuditada, Consultas, EntradaAuditoria, FabricaUnidadDeTrabajo, RegistroAuditoria,
        RegistroHechos, UnidadDeTrabajo,
    };
    use limen_dominio::auditoria::CambioCampo;
    use limen_dominio::empresa::EmpresaId;
    use limen_dominio::hecho::{Hecho, HechoId, Salida, Suceso};
    use limen_dominio::movimiento::Marca;
    use limen_dominio::operador::OperadorId;
    use limen_dominio::presencia::Via;
    use uuid::Uuid;

    use super::*;

    fn en() -> DateTime<Utc> {
        "2026-10-09T17:00:00Z".parse().unwrap()
    }

    /// Un almacén con un hecho (`hecho:…01`) y una entrada de auditoría
    /// (`auditoria:…02`).
    async fn con_un_hecho_y_una_auditoria() -> AlmacenSurreal {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let operador = OperadorId::desde_uuid(Uuid::from_u128(9));
        let mut uow = almacen.nueva();
        uow.hechos().anotar(Hecho::restaurar(
            HechoId::desde_uuid(Uuid::from_u128(1)),
            Suceso::Salida(Salida {
                via: Via::Contratista,
                registro: Uuid::from_u128(5),
                marca: Marca { en: en(), operador },
            }),
        ));
        uow.auditoria().anotar(EntradaAuditoria {
            id_entrada: Uuid::from_u128(2),
            registro: RegistroAuditado::Empresa(EmpresaId::desde_uuid(Uuid::from_u128(3))),
            accion: AccionAuditada::Alta,
            cambios: vec![CambioCampo {
                campo: "nombre",
                antes: String::new(),
                despues: "ACME".to_owned(),
            }],
            operador,
            en: en(),
        });
        uow.confirmar().await.unwrap();
        almacen
    }

    /// Ejecuta una sentencia y devuelve sus errores como texto.
    async fn errores_de(almacen: &AlmacenSurreal, sentencia: &str) -> Vec<String> {
        let mut respuesta = almacen.db().query(sentencia).await.unwrap();
        respuesta
            .take_errors()
            .into_values()
            .map(|error| error.to_string())
            .collect()
    }

    async fn cuantos(almacen: &AlmacenSurreal, tabla: &str) -> usize {
        let mut respuesta = almacen
            .db()
            .query(format!("SELECT VALUE id FROM {tabla}"))
            .await
            .unwrap();
        let ids: Vec<surrealdb::types::RecordId> = respuesta.take(0).unwrap();
        ids.len()
    }

    #[tokio::test]
    async fn la_base_no_deja_editar_ni_borrar_un_hecho() {
        let almacen = con_un_hecho_y_una_auditoria().await;
        let hecho = "hecho:u'00000000-0000-0000-0000-000000000001'";
        for sentencia in [
            format!("UPDATE {hecho} SET via = 'PROVEEDOR';"),
            // Un reemplazo completo y válido: lo frena el evento, no el tipo.
            format!(
                "UPSERT {hecho} CONTENT {{ tipo: 'SALIDA', via: 'PROVEEDOR', \
                 registro: u'00000000-0000-0000-0000-000000000005', \
                 en: d'2026-10-09T17:00:00Z', \
                 operador: u'00000000-0000-0000-0000-000000000009' }};"
            ),
            format!("DELETE {hecho};"),
            "DELETE hecho;".to_owned(),
        ] {
            let errores = errores_de(&almacen, &sentencia).await;
            assert!(
                errores
                    .iter()
                    .any(|e| e.contains("no se editan ni se borran") || e.contains("is readonly")),
                "la base rechaza {sentencia}: {errores:?}"
            );
        }
        let hechos = almacen.hechos_de(Uuid::from_u128(5)).await.unwrap();
        assert_eq!(hechos.len(), 1, "el hecho sigue ahí");
        assert_eq!(
            hechos.first().map(Hecho::via),
            Some(Via::Contratista),
            "y sin cambios"
        );
    }

    #[tokio::test]
    async fn la_base_no_deja_editar_ni_borrar_la_auditoria() {
        let almacen = con_un_hecho_y_una_auditoria().await;
        let entrada = "auditoria:u'00000000-0000-0000-0000-000000000002'";
        for sentencia in [
            format!("UPDATE {entrada} SET accion = 'edicion';"),
            format!("DELETE {entrada};"),
            "DELETE auditoria;".to_owned(),
        ] {
            let errores = errores_de(&almacen, &sentencia).await;
            assert!(
                errores
                    .iter()
                    .any(|e| e.contains("no se edita ni se borra")),
                "la base rechaza {sentencia}: {errores:?}"
            );
        }
        assert_eq!(
            cuantos(&almacen, "auditoria").await,
            1,
            "la entrada sigue ahí"
        );
    }

    #[tokio::test]
    async fn las_demas_tablas_si_se_pueden_borrar() {
        // El evento es sólo de `hecho` y `auditoria`: el estado derivado
        // (presencias, préstamos) se crea y se borra con normalidad.
        let almacen = con_un_hecho_y_una_auditoria().await;
        let errores = errores_de(
            &almacen,
            "CREATE presencia:x CONTENT { via: 'CONTRATISTA', desde: d'2026-10-09T08:00:00Z' }; DELETE presencia:x;",
        )
        .await;
        assert_eq!(
            errores,
            Vec::<String>::new(),
            "la presencia se borra sin problema"
        );
    }
}
