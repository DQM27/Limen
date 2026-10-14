//! Subida del equipo a la nube: lo que el equipo registra, con los mismos IDs,
//! sobre el mismo esquema (paso 3 de `docs/nube.md`).
//!
//! Un solo camino, que usan la app (cada pocos segundos) y la herramienta
//! `limen-nube subir`: **vacía la bandeja de salida del equipo** (patrón «outbox», como
//! `cola_salida` en Lattis; ver `limen_infra_surreal::AlmacenSurreal::pendientes_de_la_bandeja`).
//! La bandeja la llenan los eventos de la propia base con cada escritura del equipo, y lo que
//! baja de la nube no entra: no hay rebote. Se puede repetir sin duplicar.
//!
//! Sólo crea o reemplaza: **nunca borra en la nube** lo que no esté anotado como borrado, porque
//! en la nube también está lo de los demás equipos.
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

/// Lo que el equipo sólo baja y nunca sube: los usuarios son globales y los administra la nube
/// (el equipo sólo inicia sesión y cambia su propia clave).
pub const SOLO_BAJADA: [(&str, Copia); 1] = [("usuario", Copia::Reemplazar)];

/// Cómo se copia una tabla: reemplazando o sólo agregando.
pub fn copia_de(tabla: &str) -> Copia {
    TABLAS
        .iter()
        .chain(SOLO_BAJADA.iter())
        .find(|(nombre, _)| *nombre == tabla)
        .map_or(Copia::Reemplazar, |(_, copia)| *copia)
}

impl Nube {
    /// Sube lo que hay en la bandeja de salida del equipo: las altas, los cambios y los borrados
    /// que el equipo hizo y todavía no mandó. Devuelve cuántos cambios subió.
    ///
    /// Las tablas van en el orden de [`TABLAS`] (los catálogos antes que lo que los usa), de a
    /// [`POR_LOTE`]. Un lote se quita de la bandeja **después** de confirmarse en la nube: si algo
    /// falla, sigue en la bandeja y se reintenta en la próxima vuelta; y si una fila cambió
    /// mientras se mandaba, sigue pendiente con su versión nueva.
    ///
    /// La primera vez, la bandeja recibe lo que el equipo ya tenía antes de existir (una base de
    /// antes, o lo que se registró sin conexión antes de vincularse).
    pub async fn subir_cambios(&self, almacen: &AlmacenSurreal) -> Result<usize, ErrorNube> {
        let error = |e: &dyn std::fmt::Display| ErrorNube::Subida(e.to_string());
        almacen.sembrar_la_bandeja().await.map_err(|e| error(&e))?;
        let mut subidos = 0;
        for (tabla, copia) in TABLAS {
            let error = |e: &dyn std::fmt::Display| ErrorNube::Subida(format!("{tabla}: {e}"));
            loop {
                let pendientes = almacen
                    .pendientes_de_la_bandeja(tabla, POR_LOTE)
                    .await
                    .map_err(|e| error(&e))?;
                if pendientes.is_empty() {
                    break;
                }
                let cambios: Vec<Cambio> = pendientes.iter().map(|p| p.cambio.clone()).collect();
                subidos += cambios.len();
                self.aplicar(tabla, copia, cambios, almacen)
                    .await
                    .map_err(|e| error(&e))?;
                almacen
                    .quitar_de_la_bandeja(&pendientes)
                    .await
                    .map_err(|e| error(&e))?;
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

    /// La bandeja de salida (en `limen_infra_surreal`) y la subida tienen que hablar de las mismas
    /// tablas: si una tabla se sube pero no tiene su evento, sus cambios nunca llegarían.
    #[test]
    fn la_bandeja_y_la_subida_hablan_de_las_mismas_tablas() {
        let de_la_subida: Vec<&str> = TABLAS.iter().map(|(tabla, _)| *tabla).collect();
        assert_eq!(de_la_subida, limen_infra_surreal::TABLAS_QUE_SUBEN);
    }

    /// El orden real de cada vuelta de la app: sube y después baja.
    async fn una_vuelta(nube: &Nube, equipo: &AlmacenSurreal) -> (usize, usize) {
        let subidos = nube.subir_cambios(equipo).await.unwrap();
        let bajadas = nube.bajar_cambios(equipo).await.unwrap();
        (subidos, bajadas)
    }

    /// Una instalación nueva baja el catálogo entero de la nube y **no** lo vuelve a subir: eran
    /// miles de escrituras de rebote que además ensuciaban el diario de la nube.
    #[tokio::test]
    async fn lo_que_un_equipo_nuevo_baja_de_la_nube_no_vuelve_a_subir() {
        let nube = nube().await;
        let otro = equipo_con_datos().await;
        nube.subir_cambios(&otro).await.unwrap();
        let diario_antes = en_la_nube(&nube, "cambio").await;

        let nuevo = AlmacenSurreal::en_memoria().await.unwrap();
        let (subidos, bajadas) = una_vuelta(&nube, &nuevo).await;
        assert_eq!(subidos, 0, "una instalación nueva no tiene nada que subir");
        assert!(bajadas > 0, "baja lo de la nube");

        assert_eq!(
            nube.subir_cambios(&nuevo).await.unwrap(),
            0,
            "la vuelta siguiente no devuelve lo que acaba de bajar"
        );
        assert_eq!(
            en_la_nube(&nube, "cambio").await,
            diario_antes,
            "ni una escritura de rebote en la nube"
        );
        assert_eq!(
            nube.bajar_cambios(&nuevo).await.unwrap(),
            0,
            "y todo quedó al día"
        );
    }

    /// Lo que el operador registra después de bajar sí sube, y sólo eso.
    #[tokio::test]
    async fn despues_de_bajar_lo_que_registra_el_operador_si_sube() {
        let nube = nube().await;
        let otro = equipo_con_datos().await;
        nube.subir_cambios(&otro).await.unwrap();
        let nuevo = AlmacenSurreal::en_memoria().await.unwrap();
        una_vuelta(&nube, &nuevo).await;

        let mut uow = nuevo.nueva();
        uow.gafetes().agregar(
            &Gafete::registrar(
                TipoGafete::Contratista,
                NumeroGafete::nuevo(30).unwrap(),
                false,
            )
            .unwrap(),
        );
        uow.confirmar().await.unwrap();

        assert_eq!(
            nube.subir_cambios(&nuevo).await.unwrap(),
            1,
            "sólo el gafete nuevo, no lo que se bajó"
        );
        assert_eq!(
            cuantos(
                &nube,
                "SELECT VALUE id FROM gafete:`PRUEBAS-CONTRATISTA-30`"
            )
            .await,
            1
        );
    }

    /// Un equipo que trabajó sin conexión antes de vincularse conserva su trabajo: lo suyo sube y
    /// lo que baja de los demás no se devuelve.
    #[tokio::test]
    async fn lo_que_se_hizo_sin_conexion_sube_y_lo_que_baja_no_se_devuelve() {
        let nube = nube().await;
        let otro = AlmacenSurreal::en_memoria().await.unwrap();
        let mut uow = otro.nueva();
        uow.gafetes().agregar(
            &Gafete::registrar(
                TipoGafete::Contratista,
                NumeroGafete::nuevo(26).unwrap(),
                false,
            )
            .unwrap(),
        );
        uow.confirmar().await.unwrap();
        nube.subir_cambios(&otro).await.unwrap();

        // Este equipo tiene su gafete 25, su presencia y su hecho de antes de vincularse.
        let equipo = equipo_con_datos().await;
        let (subidos, bajadas) = una_vuelta(&nube, &equipo).await;
        assert!(subidos > 0, "lo suyo sube");
        assert!(bajadas > 0, "baja el gafete 26 del otro equipo");
        assert_eq!(
            en_la_nube(&nube, "presencia").await,
            1,
            "su presencia llegó"
        );
        assert_eq!(
            en_la_nube(&nube, "gafete").await,
            2,
            "y los dos gafetes están"
        );

        assert_eq!(
            nube.subir_cambios(&equipo).await.unwrap(),
            0,
            "el gafete 26 no vuelve a la nube: ya era de allá"
        );
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
