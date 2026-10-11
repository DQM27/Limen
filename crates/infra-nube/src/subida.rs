//! Subida de la base de un equipo a la nube: copia lo que el equipo tiene,
//! con los mismos IDs, sobre el mismo esquema.
//!
//! Es la carga inicial (pasos 3 y 5 de `docs/nube.md`): se puede repetir y
//! no duplica nada. La sincronización continua (subir lo pendiente al
//! registrar, bajar en vivo) se construye después, encima de esto.
//!
//! No sube lo que es del equipo y no de la nube: los intentos de inicio de
//! sesión, el reloj del equipo y los usuarios (los usuarios son globales y
//! vienen de la nube; los de un equipo de desarrollo son de prueba).

use limen_infra_surreal::AlmacenSurreal;
use surrealdb::types::Value;

use crate::{ErrorNube, Nube};

/// Cuántas filas se mandan por consulta.
const POR_LOTE: usize = 200;

/// Cómo se copia cada tabla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Copia {
    /// Catálogos y registros que cambian (un ingreso recibe su salida): se
    /// crean o se reemplazan por ID.
    Reemplazar,
    /// Estado que se borra al terminar (quién está adentro, los préstamos
    /// abiertos): lo de la unidad en la nube se deja igual al del equipo, y
    /// así no queda adentro alguien que ya salió.
    Estado,
    /// Hechos y auditoría: nunca se editan. Se agregan los que faltan y los
    /// que ya están no se tocan.
    Agregar,
}

/// Las tablas que se suben y cómo, en orden: los catálogos antes que lo que
/// los usa.
pub const TABLAS: [(&str, Copia); 14] = [
    ("empresa", Copia::Reemplazar),
    ("empresa_proveedora", Copia::Reemplazar),
    ("contratista", Copia::Reemplazar),
    ("personal_kof", Copia::Reemplazar),
    ("gafete", Copia::Reemplazar),
    ("ingreso_contratista", Copia::Reemplazar),
    ("ingreso_proveedor", Copia::Reemplazar),
    ("ingreso_correo", Copia::Reemplazar),
    ("prestamo_kof", Copia::Reemplazar),
    ("prestamo_gafete", Copia::Estado),
    ("presencia", Copia::Estado),
    ("personal_kof_con_prestamo", Copia::Estado),
    ("hecho", Copia::Agregar),
    ("auditoria", Copia::Agregar),
];

/// Cuántas filas de una tabla se subieron.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subida {
    pub tabla: &'static str,
    pub filas: usize,
}

impl Nube {
    /// Sube a la nube todo lo que tiene la base de este equipo (ver
    /// [`TABLAS`]). Si falla una tabla, se detiene ahí: lo ya subido queda y
    /// repetir la subida no lo duplica.
    pub async fn subir_equipo(&self, almacen: &AlmacenSurreal) -> Result<Vec<Subida>, ErrorNube> {
        let mut subidas = Vec::with_capacity(TABLAS.len());
        for (tabla, copia) in TABLAS {
            let filas = almacen
                .filas(tabla)
                .await
                .map_err(|e| ErrorNube::Subida(format!("{tabla}: {e}")))?;
            let filas: Vec<Value> = if tabla == "hecho" {
                filas
                    .into_iter()
                    .map(|fila| con_origen(fila, almacen))
                    .collect()
            } else {
                filas
            };
            let cuantas = filas.len();
            self.copiar(tabla, copia, filas, almacen.unidad().as_str())
                .await
                .map_err(|e| ErrorNube::Subida(format!("{tabla}: {e}")))?;
            subidas.push(Subida {
                tabla,
                filas: cuantas,
            });
        }
        Ok(subidas)
    }

    async fn copiar(
        &self,
        tabla: &'static str,
        copia: Copia,
        filas: Vec<Value>,
        unidad: &str,
    ) -> Result<(), surrealdb::Error> {
        // Las tablas salen de `TABLAS`, nunca de un dato: los valores van
        // siempre como parámetros.
        if copia == Copia::Estado {
            self.db
                .query(format!(
                    "BEGIN TRANSACTION; DELETE {tabla} WHERE unidad = $unidad; \
                     FOR $fila IN $filas {{ CREATE $fila.id CONTENT $fila; }}; \
                     COMMIT TRANSACTION;"
                ))
                .bind(("unidad", unidad.to_owned()))
                .bind(("filas", filas))
                .await?
                .check()?;
            return Ok(());
        }
        let sentencia = match copia {
            Copia::Agregar => format!("INSERT IGNORE INTO {tabla} $filas;"),
            _ => "FOR $fila IN $filas { UPSERT $fila.id CONTENT $fila; };".to_owned(),
        };
        for lote in filas.chunks(POR_LOTE) {
            self.db
                .query(sentencia.as_str())
                .bind(("filas", lote.to_vec()))
                .await?
                .check()?;
        }
        Ok(())
    }
}

/// Un hecho guardado antes de que los hechos llevaran su origen se registró
/// en este equipo (ver `limen_infra_surreal`): sube con ese origen, que la
/// nube exige.
fn con_origen(fila: Value, almacen: &AlmacenSurreal) -> Value {
    match fila {
        Value::Object(mut campos) if !campos.contains_key("unidad") => {
            campos.insert("unidad", almacen.origen().unidad.to_string());
            campos.insert("equipo", almacen.origen().equipo.uuid());
            Value::Object(campos)
        }
        otra => otra,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use limen_aplicacion::puertos::{
        FabricaUnidadDeTrabajo, RegistroHechos, RepositorioGafetes, RepositorioPresencias,
        UnidadDeTrabajo,
    };
    use limen_dominio::cedula::Cedula;
    use limen_dominio::gafete::{Gafete, NumeroGafete, TipoGafete};
    use limen_dominio::hecho::{CalidadHora, Hecho, HechoId, Salida, Suceso};
    use limen_dominio::movimiento::Marca;
    use limen_dominio::operador::OperadorId;
    use limen_dominio::presencia::{Identidad, Via};
    use uuid::Uuid;

    use super::*;
    use crate::ConfigNube;

    fn en() -> DateTime<Utc> {
        "2026-10-09T08:00:00Z".parse().unwrap()
    }

    fn cedula() -> Identidad {
        Identidad::from(&Cedula::normalizar("111111111").unwrap())
    }

    /// Un equipo con un gafete prestado, una persona adentro y un hecho.
    async fn equipo_con_datos() -> AlmacenSurreal {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let numero = NumeroGafete::nuevo(25).unwrap();
        let mut uow = almacen.nueva();
        uow.gafetes()
            .agregar(&Gafete::registrar(TipoGafete::Contratista, numero, false).unwrap());
        uow.gafetes()
            .anotar_prestamo(TipoGafete::Contratista, numero, en());
        uow.presencias()
            .anotar_entrada(&cedula(), Via::Contratista, en());
        uow.hechos().anotar(Hecho::restaurar(
            HechoId::desde_uuid(Uuid::from_u128(1)),
            Suceso::Salida(Salida {
                via: Via::Contratista,
                registro: Uuid::from_u128(5),
                marca: Marca {
                    en: en(),
                    operador: OperadorId::desde_uuid(Uuid::from_u128(9)),
                },
            }),
            almacen.origen().clone(),
            CalidadHora {
                confiable: true,
                hora_equipo: en(),
            },
        ));
        uow.confirmar().await.unwrap();
        almacen
    }

    async fn nube() -> Nube {
        let nube = Nube::conectar(&ConfigNube {
            url: "mem://".to_owned(),
            credenciales: None,
        })
        .await
        .unwrap();
        nube.aplicar_esquema().await.unwrap();
        nube
    }

    /// Cuántos registros hay en la nube con esta consulta (`SELECT VALUE id …`).
    async fn cuantos(nube: &Nube, consulta: &str) -> usize {
        let mut respuesta = nube.db.query(consulta).await.unwrap();
        let ids: Vec<surrealdb::types::RecordId> = respuesta.take(0).unwrap();
        ids.len()
    }

    async fn en_la_nube(nube: &Nube, tabla: &str) -> usize {
        cuantos(nube, &format!("SELECT VALUE id FROM {tabla}")).await
    }

    fn filas_de(subidas: &[Subida], tabla: &str) -> usize {
        subidas
            .iter()
            .find(|subida| subida.tabla == tabla)
            .map_or(0, |subida| subida.filas)
    }

    #[tokio::test]
    async fn la_base_del_equipo_sube_con_los_mismos_ids() {
        let equipo = equipo_con_datos().await;
        let nube = nube().await;

        let subidas = nube.subir_equipo(&equipo).await.unwrap();

        assert_eq!(
            subidas.len(),
            TABLAS.len(),
            "dice cuánto subió de cada tabla"
        );
        assert_eq!(filas_de(&subidas, "gafete"), 1);
        assert_eq!(filas_de(&subidas, "hecho"), 1);
        assert_eq!(
            cuantos(
                &nube,
                "SELECT VALUE id FROM gafete:`PRUEBAS-CONTRATISTA-25`"
            )
            .await,
            1,
            "con la misma clave"
        );
        assert_eq!(en_la_nube(&nube, "prestamo_gafete").await, 1);
        assert_eq!(en_la_nube(&nube, "presencia").await, 1);
        assert_eq!(
            cuantos(
                &nube,
                "SELECT VALUE id FROM hecho:u'00000000-0000-0000-0000-000000000001'"
            )
            .await,
            1,
            "el hecho, con su mismo ID"
        );
    }

    #[tokio::test]
    async fn subir_otra_vez_no_duplica_y_quien_salio_no_queda_adentro() {
        let equipo = equipo_con_datos().await;
        let nube = nube().await;
        nube.subir_equipo(&equipo).await.unwrap();

        // La persona sale y devuelve el gafete en el equipo.
        let mut uow = equipo.nueva();
        uow.presencias().anotar_salida(&cedula());
        uow.gafetes()
            .anotar_devolucion(TipoGafete::Contratista, NumeroGafete::nuevo(25).unwrap());
        uow.confirmar().await.unwrap();
        nube.subir_equipo(&equipo).await.unwrap();

        assert_eq!(en_la_nube(&nube, "gafete").await, 1, "no duplica");
        assert_eq!(en_la_nube(&nube, "hecho").await, 1, "ni los hechos");
        assert_eq!(
            en_la_nube(&nube, "presencia").await,
            0,
            "quien salió en el equipo ya no está adentro en la nube"
        );
        assert_eq!(
            en_la_nube(&nube, "prestamo_gafete").await,
            0,
            "y el gafete ya no está prestado"
        );
    }
}
