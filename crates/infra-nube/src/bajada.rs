//! Bajada de la nube al equipo: lo que registran los demás equipos.
//!
//! Cada equipo baja los catálogos completos (son globales) y, de lo que es de
//! una unidad operativa, sólo lo de la suya (M4). Usa el registro de cambios
//! de la nube, que es el mismo esquema que el del equipo: pide lo anotado
//! desde la última vez que bajó. Así lo mismo sirve en vivo (cada pocos
//! segundos) y al volver la conexión.
//!
//! Lo que llega se aplica sin escribir lo que ya está igual
//! (`AlmacenSurreal::aplicar_de_la_nube`): por eso un cambio no rebota entre
//! los equipos y la nube.

use limen_infra_surreal::{AlmacenSurreal, Cambio, TABLAS_DE_LA_UNIDAD, entradas};
use surrealdb::types::Value;

use crate::subida::{Copia, SOLO_BAJADA, TABLAS};
use crate::{ErrorNube, Nube};

/// Cuántas entradas del registro de la nube se piden por consulta.
const POR_LOTE: usize = 200;

impl Nube {
    /// Baja al equipo lo que cambió en la nube desde la última vez y
    /// devuelve cuántas filas del equipo cambiaron de verdad (cero si todo
    /// lo que llegó ya estaba igual).
    ///
    /// La primera vez, cada tabla baja entera (para un equipo nuevo es la
    /// carga inicial) y su registro se repasa desde el principio.
    pub async fn bajar_cambios(&self, almacen: &AlmacenSurreal) -> Result<usize, ErrorNube> {
        let unidad = almacen.unidad().as_str().to_owned();
        let mut cambiadas = 0;
        // Los usuarios primero: sin ellos nadie puede iniciar sesión.
        for (tabla, copia) in SOLO_BAJADA.into_iter().chain(TABLAS) {
            let error = |e: &dyn std::fmt::Display| ErrorNube::Bajada(format!("{tabla}: {e}"));
            let inmutable = copia == Copia::Agregar;
            let de_la_unidad = es_de_la_unidad(tabla);
            let ya_bajado = almacen.bajado_hasta(tabla).await.map_err(|e| error(&e))?;
            if ya_bajado.is_none() {
                let filas = self
                    .filas_de_la_nube(tabla, de_la_unidad.then_some(unidad.as_str()))
                    .await
                    .map_err(|e| error(&e))?;
                for lote in filas.chunks(POR_LOTE) {
                    let cambios = lote.iter().cloned().map(Cambio::Guardada).collect();
                    cambiadas += almacen
                        .aplicar_de_la_nube(cambios, inmutable)
                        .await
                        .map_err(|e| error(&e))?;
                }
                almacen
                    .anotar_bajado_hasta(tabla, 0)
                    .await
                    .map_err(|e| error(&e))?;
            }
            let mut desde = ya_bajado.map_or(0, |hasta| hasta + 1);
            loop {
                let crudo = self
                    .registro_de_la_nube(tabla, desde)
                    .await
                    .map_err(|e| error(&e))?;
                let lote = entradas(crudo).map_err(|e| error(&e))?;
                let Some(ultima) = lote.last().map(|entrada| entrada.numero) else {
                    break;
                };
                for entrada in lote {
                    let cambios = entrada
                        .cambios
                        .into_iter()
                        .filter(|cambio| !de_la_unidad || es_de(cambio, &unidad))
                        .collect();
                    cambiadas += almacen
                        .aplicar_de_la_nube(cambios, inmutable)
                        .await
                        .map_err(|e| error(&e))?;
                }
                almacen
                    .anotar_bajado_hasta(tabla, ultima)
                    .await
                    .map_err(|e| error(&e))?;
                desde = ultima + 1;
            }
        }
        Ok(cambiadas)
    }

    /// Todas las filas de una tabla en la nube; con `unidad`, sólo las de
    /// esa unidad.
    async fn filas_de_la_nube(
        &self,
        tabla: &'static str,
        unidad: Option<&str>,
    ) -> Result<Vec<Value>, surrealdb::Error> {
        let mut respuesta = match unidad {
            Some(unidad) => {
                self.db
                    .query(format!(
                        "SELECT * FROM {tabla} WHERE unidad = $unidad ORDER BY id"
                    ))
                    .bind(("unidad", unidad.to_owned()))
                    .await?
            }
            None => {
                self.db
                    .query(format!("SELECT * FROM {tabla} ORDER BY id"))
                    .await?
            }
        };
        respuesta.take(0)
    }

    async fn registro_de_la_nube(
        &self,
        tabla: &'static str,
        desde: i64,
    ) -> Result<Value, surrealdb::Error> {
        // `SINCE` y `LIMIT` sólo aceptan literales: son números, y la tabla
        // sale de `TABLAS`, nunca de un dato.
        self.db
            .query(format!(
                "SHOW CHANGES FOR TABLE {tabla} SINCE {desde} LIMIT {POR_LOTE}"
            ))
            .await?
            .take(0)
    }
}

/// Si las filas de la tabla son de una unidad operativa (y no globales).
pub fn es_de_la_unidad(tabla: &str) -> bool {
    tabla == "hecho" || TABLAS_DE_LA_UNIDAD.contains(&tabla)
}

/// Si un cambio de una tabla de la unidad es de `unidad`. Un borrado no dice
/// de qué unidad era: se deja pasar, y sólo borra si la fila está en el
/// equipo, donde todo es de su unidad.
fn es_de(cambio: &Cambio, unidad: &str) -> bool {
    match cambio {
        Cambio::Guardada(Value::Object(campos)) => {
            matches!(campos.get("unidad"), Some(Value::String(suya)) if suya == unidad)
        }
        Cambio::Guardada(_) => false,
        Cambio::Borrada(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use limen_aplicacion::puertos::{
        FabricaUnidadDeTrabajo, RepositorioPresencias, UnidadDeTrabajo,
    };
    use limen_dominio::cedula::Cedula;
    use limen_dominio::presencia::{Identidad, Via};

    use super::*;
    use crate::ConfigNube;

    fn en() -> DateTime<Utc> {
        "2026-10-09T08:00:00Z".parse().unwrap()
    }

    fn persona(cedula: &str) -> Identidad {
        Identidad::from(&Cedula::normalizar(cedula).unwrap())
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

    async fn entra(equipo: &AlmacenSurreal, cedula: &str) {
        let mut uow = equipo.nueva();
        uow.presencias()
            .anotar_entrada(&persona(cedula), Via::Contratista, en());
        uow.confirmar().await.unwrap();
    }

    async fn adentro(equipo: &AlmacenSurreal, cedula: &str) -> bool {
        equipo
            .nueva()
            .presencias()
            .via_adentro(&persona(cedula))
            .await
            .unwrap()
            .is_some()
    }

    /// Una vuelta de la sincronización de un equipo: sube y después baja.
    async fn vuelta(nube: &Nube, equipo: &AlmacenSurreal) -> usize {
        nube.subir_cambios(equipo).await.unwrap();
        nube.bajar_cambios(equipo).await.unwrap()
    }

    #[tokio::test]
    async fn lo_que_registra_un_equipo_le_llega_al_otro_sin_rebotar() {
        let nube = nube().await;
        let a = AlmacenSurreal::en_memoria().await.unwrap();
        let b = AlmacenSurreal::en_memoria().await.unwrap();
        vuelta(&nube, &a).await;
        vuelta(&nube, &b).await;

        entra(&a, "111111111").await;
        vuelta(&nube, &a).await;
        assert!(vuelta(&nube, &b).await > 0, "a B le llega algo nuevo");
        assert!(
            adentro(&b, "111111111").await,
            "B ve adentro a quien entró en A"
        );

        // Unas vueltas más: nadie vuelve a escribir nada.
        for _ in 0..2 {
            assert_eq!(
                vuelta(&nube, &a).await,
                0,
                "A no recibe su propio cambio de vuelta"
            );
            assert_eq!(vuelta(&nube, &b).await, 0, "B ya lo tiene");
        }

        // La salida, registrada en B, también le llega a A.
        let mut uow = b.nueva();
        uow.presencias().anotar_salida(&persona("111111111"));
        uow.confirmar().await.unwrap();
        vuelta(&nube, &b).await;
        assert!(vuelta(&nube, &a).await > 0);
        assert!(!adentro(&a, "111111111").await, "A sabe que ya salió");
    }

    const USUARIO: &str = "0199c7a2-0000-7000-8000-0000000000aa";

    async fn usuario_en_la_nube(nube: &Nube, nombre: &str) {
        nube.db
            .query(
                "UPSERT type::record('usuario', <uuid>$id) CONTENT { cedula: '155824395105', \
                 nombre: $nombre, activo: true, clave: 'hash-de-prueba', \
                 debe_cambiar_clave: false };",
            )
            .bind(("id", USUARIO.to_owned()))
            .bind(("nombre", nombre.to_owned()))
            .await
            .unwrap()
            .check()
            .unwrap();
    }

    async fn nombres_de_usuarios(equipo: &AlmacenSurreal) -> Vec<String> {
        equipo
            .filas("usuario")
            .await
            .unwrap()
            .into_iter()
            .filter_map(|fila| match fila {
                Value::Object(campos) => match campos.get("nombre") {
                    Some(Value::String(nombre)) => Some(nombre.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn los_usuarios_de_la_nube_bajan_al_equipo_y_sus_cambios_tambien() {
        let nube = nube().await;
        let equipo = AlmacenSurreal::en_memoria().await.unwrap();
        usuario_en_la_nube(&nube, "ANA MORA").await;

        vuelta(&nube, &equipo).await;
        assert_eq!(
            nombres_de_usuarios(&equipo).await,
            vec!["ANA MORA".to_owned()],
            "la carga inicial trae al usuario de la nube"
        );

        usuario_en_la_nube(&nube, "ANA MORA SOTO").await;
        vuelta(&nube, &equipo).await;
        assert_eq!(
            nombres_de_usuarios(&equipo).await,
            vec!["ANA MORA SOTO".to_owned()],
            "un cambio posterior llega por el registro de cambios"
        );
    }

    #[tokio::test]
    async fn lo_de_otra_unidad_no_baja() {
        let nube = nube().await;
        nube.db
            .query(
                "CREATE presencia:`222222222` CONTENT { unidad: 'OTRA', via: 'PROVEEDOR', \
                 desde: d'2026-10-09T08:00:00Z' }",
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        let equipo = AlmacenSurreal::en_memoria().await.unwrap();

        assert_eq!(
            vuelta(&nube, &equipo).await,
            0,
            "la primera bajada no lo trae"
        );
        assert!(!adentro(&equipo, "222222222").await);

        nube.db
            .query(
                "CREATE presencia:`333333333` CONTENT { unidad: 'OTRA', via: 'PROVEEDOR', \
                 desde: d'2026-10-09T09:00:00Z' }",
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        assert_eq!(vuelta(&nube, &equipo).await, 0, "ni lo que llega después");
        assert!(!adentro(&equipo, "333333333").await);
    }

    #[tokio::test]
    async fn un_equipo_nuevo_baja_lo_que_ya_estaba_en_la_nube() {
        let nube = nube().await;
        let viejo = AlmacenSurreal::en_memoria().await.unwrap();
        entra(&viejo, "444444444").await;
        vuelta(&nube, &viejo).await;

        let nuevo = AlmacenSurreal::en_memoria().await.unwrap();
        assert!(vuelta(&nube, &nuevo).await > 0);

        assert!(
            adentro(&nuevo, "444444444").await,
            "un equipo nuevo arranca al día"
        );
    }
}
