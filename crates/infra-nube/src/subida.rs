//! Subida del equipo a la nube: lo que el equipo registra, con los mismos IDs,
//! sobre el mismo esquema (paso 3 de `docs/nube.md`).
//!
//! Un solo camino, que usan la app (cada pocos segundos) y la herramienta
//! `limen-nube subir`: la primera vez cada tabla se copia entera y después
//! sube lo anotado en el registro de cambios. Se puede repetir sin duplicar.
//!
//! La copia entera sólo crea o reemplaza: **nunca borra en la nube**, porque
//! en la nube también está lo de los demás equipos. Los borrados viajan sólo
//! por el registro de cambios, que dice exactamente qué se borró.
//!
//! No sube lo que es del equipo y no de la nube: los intentos de inicio de
//! sesión, el reloj del equipo y los usuarios (los usuarios son globales y
//! vienen de la nube; los de un equipo de desarrollo son de prueba).

use limen_infra_surreal::{AlmacenSurreal, Cambio};
use surrealdb::types::Value;

use crate::{ErrorNube, Nube};

/// Cuántas filas se mandan por consulta.
const POR_LOTE: usize = 200;

/// Cómo se copia cada tabla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Copia {
    /// Catálogos, registros que cambian (un ingreso recibe su salida) y el
    /// estado que se borra al terminar (quién está adentro, los préstamos
    /// abiertos): se crean o se reemplazan por ID, y lo borrado se borra.
    Reemplazar,
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
    ("prestamo_gafete", Copia::Reemplazar),
    ("presencia", Copia::Reemplazar),
    ("personal_kof_con_prestamo", Copia::Reemplazar),
    ("hecho", Copia::Agregar),
    ("auditoria", Copia::Agregar),
];

impl Nube {
    /// Sube lo que cambió en el equipo desde la última vez: las altas, los
    /// cambios y los borrados que anotó el registro de cambios de cada tabla
    /// (ver `limen_infra_surreal::Entrada`), en orden. Devuelve cuántos
    /// cambios subió.
    ///
    /// La primera vez, una tabla se copia entera (lo que había antes del
    /// registro de cambios no está anotado) y su registro se repasa desde el
    /// principio: todo lo que se sube se puede repetir sin duplicar nada.
    /// Si algo falla, lo que no se anotó como subido se vuelve a intentar la
    /// próxima vez.
    pub async fn subir_cambios(&self, almacen: &AlmacenSurreal) -> Result<usize, ErrorNube> {
        let mut subidos = 0;
        for (tabla, copia) in TABLAS {
            let error = |e: &dyn std::fmt::Display| ErrorNube::Subida(format!("{tabla}: {e}"));
            let ya_subido = almacen.subido_hasta(tabla).await.map_err(|e| error(&e))?;
            if ya_subido.is_none() {
                let filas = filas_para_subir(tabla, almacen).await?;
                self.copiar(tabla, copia, filas)
                    .await
                    .map_err(|e| error(&e))?;
                almacen
                    .anotar_subido_hasta(tabla, 0)
                    .await
                    .map_err(|e| error(&e))?;
            }
            let mut desde = ya_subido.map_or(0, |hasta| hasta + 1);
            loop {
                let entradas = almacen
                    .cambios_desde(tabla, desde, POR_LOTE)
                    .await
                    .map_err(|e| error(&e))?;
                let Some(ultima) = entradas.last().map(|entrada| entrada.numero) else {
                    break;
                };
                for entrada in entradas {
                    subidos += entrada.cambios.len();
                    self.aplicar(tabla, copia, entrada.cambios, almacen)
                        .await
                        .map_err(|e| error(&e))?;
                }
                almacen
                    .anotar_subido_hasta(tabla, ultima)
                    .await
                    .map_err(|e| error(&e))?;
                desde = ultima + 1;
            }
        }
        Ok(subidos)
    }

    /// Aplica en la nube los cambios de una transacción del equipo, juntos.
    async fn aplicar(
        &self,
        tabla: &'static str,
        copia: Copia,
        cambios: Vec<Cambio>,
        almacen: &AlmacenSurreal,
    ) -> Result<(), surrealdb::Error> {
        let mut sentencias = vec!["BEGIN TRANSACTION;".to_owned()];
        let mut valores: Vec<(String, Value)> = Vec::new();
        for (i, cambio) in cambios.into_iter().enumerate() {
            match (cambio, copia) {
                (Cambio::Guardada(fila), Copia::Agregar) => {
                    let fila = if tabla == "hecho" {
                        con_origen(fila, almacen)
                    } else {
                        fila
                    };
                    sentencias.push(format!("INSERT IGNORE INTO {tabla} $c{i};"));
                    valores.push((format!("c{i}"), fila));
                }
                (Cambio::Guardada(fila), _) => {
                    sentencias.push(format!("UPSERT $c{i}.id CONTENT $c{i};"));
                    valores.push((format!("c{i}"), fila));
                }
                // Hechos y auditoría no se borran nunca: si apareciera un
                // borrado, no se sube.
                (Cambio::Borrada(_), Copia::Agregar) => {}
                (Cambio::Borrada(id), _) => {
                    sentencias.push(format!("DELETE $c{i};"));
                    valores.push((format!("c{i}"), Value::RecordId(id)));
                }
            }
        }
        if valores.is_empty() {
            return Ok(());
        }
        sentencias.push("COMMIT TRANSACTION;".to_owned());
        let mut consulta = self.db.query(sentencias.join("\n"));
        for (nombre, valor) in valores {
            consulta = consulta.bind((nombre, valor));
        }
        consulta.await?.check()?;
        Ok(())
    }

    /// Copia filas enteras: crea o reemplaza, nunca borra.
    async fn copiar(
        &self,
        tabla: &'static str,
        copia: Copia,
        filas: Vec<Value>,
    ) -> Result<(), surrealdb::Error> {
        // Las tablas salen de `TABLAS`, nunca de un dato: los valores van
        // siempre como parámetros.
        let sentencia = match copia {
            Copia::Agregar => format!("INSERT IGNORE INTO {tabla} $filas;"),
            Copia::Reemplazar => {
                "FOR $fila IN $filas { UPSERT $fila.id CONTENT $fila; };".to_owned()
            }
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

/// Las filas de una tabla tal como se suben: los hechos, con su origen.
async fn filas_para_subir(
    tabla: &'static str,
    almacen: &AlmacenSurreal,
) -> Result<Vec<Value>, ErrorNube> {
    let filas = almacen
        .filas(tabla)
        .await
        .map_err(|e| ErrorNube::Subida(format!("{tabla}: {e}")))?;
    Ok(if tabla == "hecho" {
        filas
            .into_iter()
            .map(|fila| con_origen(fila, almacen))
            .collect()
    } else {
        filas
    })
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

    #[tokio::test]
    async fn la_base_del_equipo_sube_con_los_mismos_ids() {
        let equipo = equipo_con_datos().await;
        let nube = nube().await;

        nube.subir_cambios(&equipo).await.unwrap();

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
    async fn la_primera_subida_de_un_equipo_no_borra_lo_de_los_demas() {
        let nube = nube().await;
        let otro = equipo_con_datos().await;
        nube.subir_cambios(&otro).await.unwrap();

        let nuevo = AlmacenSurreal::en_memoria().await.unwrap();
        nube.subir_cambios(&nuevo).await.unwrap();

        assert_eq!(
            en_la_nube(&nube, "presencia").await,
            1,
            "quien entró en el otro equipo sigue adentro en la nube"
        );
        assert_eq!(en_la_nube(&nube, "prestamo_gafete").await, 1);
    }

    #[tokio::test]
    async fn la_subida_continua_lleva_lo_que_cambia_y_nada_mas() {
        let equipo = equipo_con_datos().await;
        let nube = nube().await;

        let primera = nube.subir_cambios(&equipo).await.unwrap();
        assert!(primera > 0, "la primera vez sube lo que había");
        assert_eq!(en_la_nube(&nube, "presencia").await, 1);
        assert_eq!(
            nube.subir_cambios(&equipo).await.unwrap(),
            0,
            "sin cambios nuevos no sube nada"
        );

        // Sale y devuelve el gafete; llega otro gafete al catálogo.
        let mut uow = equipo.nueva();
        uow.presencias().anotar_salida(&cedula());
        uow.gafetes()
            .anotar_devolucion(TipoGafete::Contratista, NumeroGafete::nuevo(25).unwrap());
        uow.gafetes().agregar(
            &Gafete::registrar(
                TipoGafete::Contratista,
                NumeroGafete::nuevo(26).unwrap(),
                false,
            )
            .unwrap(),
        );
        uow.confirmar().await.unwrap();

        assert_eq!(
            nube.subir_cambios(&equipo).await.unwrap(),
            3,
            "la salida, la devolución y el gafete nuevo"
        );
        assert_eq!(
            en_la_nube(&nube, "presencia").await,
            0,
            "ya no está adentro"
        );
        assert_eq!(en_la_nube(&nube, "prestamo_gafete").await, 0);
        assert_eq!(
            en_la_nube(&nube, "gafete").await,
            2,
            "y el gafete nuevo llegó"
        );
        assert_eq!(nube.subir_cambios(&equipo).await.unwrap(), 0);
    }
}
