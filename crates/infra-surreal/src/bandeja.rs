//! La bandeja de salida del equipo (patrón «outbox», como `cola_salida` en Lattis): lo que hay
//! que mandar a la nube. Ver `esquema/equipo.surql` y `docs/nube.md`.
//!
//! La llenan los eventos de la propia base cuando algo se escribe en el equipo, dentro de la misma
//! transacción; la subida la vacía. Lo que baja de la nube se escribe con `$de_la_nube` y no
//! entra, así que no hay rebote. Es una cola «por registro»: cada fila se reescribe con su último
//! estado y una versión; el envío se confirma sólo si la versión no cambió mientras se mandaba.

use limen_aplicacion::puertos::ErrorPersistencia;
use surrealdb::types::{RecordId, SurrealValue};

use crate::almacen::AlmacenSurreal;
use crate::cambios::Cambio;
use crate::error::tecnica;

/// Las tablas del equipo que se sincronizan con la nube (suben y bajan): en cada una vive un
/// evento `salir` (`esquema/equipo.surql`). Los usuarios, el reloj y los intentos de inicio de
/// sesión son del equipo o sólo bajan, y no están aquí.
pub const TABLAS_QUE_SUBEN: [&str; 14] = [
    "empresa",
    "empresa_proveedora",
    "contratista",
    "personal_kof",
    "gafete",
    "ingreso_contratista",
    "ingreso_proveedor",
    "ingreso_correo",
    "prestamo_kof",
    "prestamo_gafete",
    "presencia",
    "personal_kof_con_prestamo",
    "hecho",
    "auditoria",
];

/// La marca de que la bandeja ya recibió lo que el equipo tenía antes de existir.
const SEMBRADA: &str = "bandeja-sembrada";

/// Algo pendiente de mandar a la nube: la fila tal como está ahora, o su borrado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pendiente {
    clave: String,
    version: i64,
    pub cambio: Cambio,
}

#[derive(Debug, SurrealValue)]
struct FilaDeSalida {
    clave: String,
    registro: RecordId,
    borrado: bool,
    version: i64,
}

impl AlmacenSurreal {
    /// La primera vez, anota en la bandeja todo lo que el equipo ya tenía antes de que existiera
    /// (una base de antes, o lo que se registró sin conexión antes de vincularse): los eventos sólo
    /// ven lo que se escribe desde que existen. Después no hace nada. Subir de más no daña: la nube
    /// recibe lo mismo que ya tiene.
    pub async fn sembrar_la_bandeja(&self) -> Result<(), ErrorPersistencia> {
        if self.cursor(SEMBRADA.to_owned()).await?.is_some() {
            return Ok(());
        }
        for tabla in TABLAS_QUE_SUBEN {
            self.db()
                .query(
                    "FOR $id IN (SELECT VALUE id FROM type::table($tabla)) { \
                     UPSERT type::record('salida', <string>$id) SET tabla = $tabla, registro = $id, \
                     borrado = false, version = (version ?? 0) + 1, en = time::now(); };",
                )
                .bind(("tabla", tabla))
                .await
                .and_then(surrealdb::IndexedResults::check)
                .map_err(tecnica)?;
        }
        self.anotar_cursor(SEMBRADA.to_owned(), 1).await
    }

    /// Lo que falta mandar de `tabla`, lo más antiguo primero, hasta `limite`. Cada uno trae la
    /// fila **como está ahora** (no como estaba cuando se anotó): sólo importa el último estado.
    pub async fn pendientes_de_la_bandeja(
        &self,
        tabla: &str,
        limite: usize,
    ) -> Result<Vec<Pendiente>, ErrorPersistencia> {
        // `LIMIT` lleva un número, nunca un dato: la tabla va como parámetro.
        let mut respuesta = self
            .db()
            .query(format!(
                "SELECT <string>record::id(id) AS clave, registro, borrado, version \
                 FROM salida WHERE tabla = $tabla ORDER BY en, clave LIMIT {limite}"
            ))
            .bind(("tabla", tabla.to_owned()))
            .await
            .map_err(tecnica)?;
        let filas: Vec<FilaDeSalida> = respuesta.take(0).map_err(tecnica)?;
        let mut pendientes = Vec::with_capacity(filas.len());
        for fila in filas {
            // Si la fila ya no está, lo que hay que mandar es su borrado.
            let cambio = match (fila.borrado, self.fila(&fila.registro).await?) {
                (false, Some(actual)) => Cambio::Guardada(actual),
                _ => Cambio::Borrada(fila.registro),
            };
            pendientes.push(Pendiente {
                clave: fila.clave,
                version: fila.version,
                cambio,
            });
        }
        Ok(pendientes)
    }

    /// Confirma que se mandaron: los quita de la bandeja, salvo los que cambiaron mientras se
    /// mandaban (su versión es otra): esos siguen pendientes.
    pub async fn quitar_de_la_bandeja(
        &self,
        enviados: &[Pendiente],
    ) -> Result<(), ErrorPersistencia> {
        if enviados.is_empty() {
            return Ok(());
        }
        let mut sentencias = vec!["BEGIN TRANSACTION;".to_owned()];
        sentencias.extend((0..enviados.len()).map(|i| {
            format!("DELETE type::record('salida', $clave_{i}) WHERE version = $version_{i};")
        }));
        sentencias.push("COMMIT TRANSACTION;".to_owned());
        let mut consulta = self.db().query(sentencias.join("\n"));
        for (i, enviado) in enviados.iter().enumerate() {
            consulta = consulta
                .bind((format!("clave_{i}"), enviado.clave.clone()))
                .bind((format!("version_{i}"), enviado.version));
        }
        consulta
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map(drop)
            .map_err(tecnica)
    }
}

#[cfg(test)]
mod tests {
    use limen_aplicacion::puertos::{
        FabricaUnidadDeTrabajo, RepositorioPresencias, UnidadDeTrabajo,
    };
    use limen_dominio::cedula::Cedula;
    use limen_dominio::presencia::{Identidad, Via};
    use surrealdb::types::Value;

    use super::*;

    fn persona() -> Identidad {
        Identidad::from(&Cedula::normalizar("111111111").unwrap())
    }

    async fn entra(almacen: &AlmacenSurreal) {
        let mut uow = almacen.nueva();
        uow.presencias().anotar_entrada(
            &persona(),
            Via::Contratista,
            "2026-10-09T08:00:00Z".parse().unwrap(),
        );
        uow.confirmar().await.unwrap();
    }

    async fn sale(almacen: &AlmacenSurreal) {
        let mut uow = almacen.nueva();
        uow.presencias().anotar_salida(&persona());
        uow.confirmar().await.unwrap();
    }

    async fn pendientes(almacen: &AlmacenSurreal) -> Vec<Pendiente> {
        almacen
            .pendientes_de_la_bandeja("presencia", 100)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn lo_que_se_escribe_en_el_equipo_queda_en_la_bandeja() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        assert!(pendientes(&almacen).await.is_empty(), "al principio, nada");

        entra(&almacen).await;

        let pendientes = pendientes(&almacen).await;
        assert_eq!(pendientes.len(), 1);
        assert!(
            matches!(pendientes[0].cambio, Cambio::Guardada(_)),
            "con la fila tal como está"
        );
    }

    #[tokio::test]
    async fn un_borrado_es_un_borrado_y_la_bandeja_guarda_el_ultimo_estado() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        entra(&almacen).await;
        sale(&almacen).await;

        let pendientes = pendientes(&almacen).await;
        assert_eq!(pendientes.len(), 1, "una sola fila por registro");
        assert!(
            matches!(pendientes[0].cambio, Cambio::Borrada(_)),
            "lo último que pasó fue el borrado"
        );
    }

    #[tokio::test]
    async fn lo_enviado_se_quita_pero_no_lo_que_cambio_mientras_se_mandaba() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        entra(&almacen).await;
        let enviados = pendientes(&almacen).await;

        // Mientras se mandaba, el operador registró la salida.
        sale(&almacen).await;
        almacen.quitar_de_la_bandeja(&enviados).await.unwrap();

        let quedan = pendientes(&almacen).await;
        assert_eq!(quedan.len(), 1, "el cambio nuevo no se pierde");
        assert!(matches!(quedan[0].cambio, Cambio::Borrada(_)));

        almacen.quitar_de_la_bandeja(&quedan).await.unwrap();
        assert!(
            pendientes(&almacen).await.is_empty(),
            "y al mandarlo, se quita"
        );
    }

    #[tokio::test]
    async fn lo_que_el_equipo_ya_tenia_antes_se_anota_una_sola_vez() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        // Una fila de antes de que existiera la bandeja: se escribe sin pasar por los eventos.
        almacen
            .db()
            .query(
                "CREATE presencia:`333333333` SET unidad = 'PRUEBAS', via = 'KOF', \
                 desde = d'2026-10-09T09:00:00Z'",
            )
            .bind(("de_la_nube", true))
            .await
            .unwrap()
            .check()
            .unwrap();
        assert!(
            pendientes(&almacen).await.is_empty(),
            "los eventos no la vieron"
        );

        almacen.sembrar_la_bandeja().await.unwrap();
        let sembradas = pendientes(&almacen).await;
        assert_eq!(sembradas.len(), 1, "ahora sí está en la bandeja");

        almacen.quitar_de_la_bandeja(&sembradas).await.unwrap();
        almacen.sembrar_la_bandeja().await.unwrap();
        assert!(
            pendientes(&almacen).await.is_empty(),
            "la siembra es de una sola vez"
        );
    }

    #[tokio::test]
    async fn cada_tabla_que_se_sincroniza_tiene_su_evento() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        for tabla in TABLAS_QUE_SUBEN {
            let mut respuesta = almacen
                .db()
                .query(format!("INFO FOR TABLE {tabla}"))
                .await
                .unwrap();
            let info: Value = respuesta.take(0).unwrap();
            assert!(
                format!("{info:?}").contains("salir"),
                "{tabla} no anota en la bandeja"
            );
        }
    }
}
