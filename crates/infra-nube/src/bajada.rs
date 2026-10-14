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

use limen_infra_surreal::{AlmacenSurreal, Cambio, TABLAS_DE_LA_UNIDAD, tabla_de};
use surrealdb::types::{Number, RecordId, Value};

use crate::subida::{Copia, SOLO_BAJADA, TABLAS, copia_de};
use crate::{ErrorNube, Nube};

/// Cuántas entradas del registro de la nube se piden por consulta.
const POR_LOTE: usize = 200;
/// Cuánto se vuelve a leer del diario antes del último cursor (microsegundos): 60 segundos.
const MARGEN_DEL_DIARIO: i64 = 60_000_000;

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
        let tablas: Vec<(&'static str, Copia)> = SOLO_BAJADA.into_iter().chain(TABLAS).collect();
        // 1. La primera vez, cada tabla baja entera (para un equipo nuevo es la carga inicial).
        for (tabla, copia) in &tablas {
            let error = |e: &dyn std::fmt::Display| ErrorNube::Bajada(format!("{tabla}: {e}"));
            let inmutable = *copia == Copia::Agregar;
            if almacen
                .bajado_hasta(tabla)
                .await
                .map_err(|e| error(&e))?
                .is_none()
            {
                let filas = self
                    .filas_de_la_nube(tabla, es_de_la_unidad(tabla).then_some(unidad.as_str()))
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
        }
        // 2. Después, el diario de cambios de la nube (`cambio`): qué filas cambiaron desde la
        //    última vez. Un equipo con token sólo recibe las anotaciones de su unidad y las
        //    globales (los permisos de la tabla); el registro de cambios de SurrealDB le está
        //    prohibido. Ver `docs/nube.md`, sección 13.
        let error = |e: &dyn std::fmt::Display| ErrorNube::Bajada(e.to_string());
        let nombres: Vec<&'static str> = tablas.iter().map(|(tabla, _)| *tabla).collect();
        let hasta_ahora = almacen
            .bajado_hasta_en_el_diario()
            .await
            .map_err(|e| error(&e))?;
        // Se vuelve a leer un margen antes del cursor: dos transacciones que confirman casi a la
        // vez pueden anotarse en el diario en otro orden que su hora, y lo que ya está igual no
        // se escribe, así que repasar un poco es gratis.
        let mut desde = hasta_ahora.map_or(0, |hasta| (hasta - MARGEN_DEL_DIARIO).max(0));
        let mut mayor = hasta_ahora.unwrap_or(0);
        loop {
            let anotaciones = self.diario_desde(desde).await.map_err(|e| error(&e))?;
            let Some(ultima) = anotaciones.iter().map(|a| a.micros).max() else {
                break;
            };
            let completa = anotaciones.len() >= POR_LOTE;
            let cambios = self.cambios_de(anotaciones).await.map_err(|e| error(&e))?;
            for (tabla, cambios) in por_tabla(cambios, &nombres) {
                let de_la_unidad = es_de_la_unidad(tabla);
                let cambios: Vec<Cambio> = cambios
                    .into_iter()
                    .filter(|cambio| !de_la_unidad || es_de(cambio, &unidad))
                    .collect();
                let inmutable = copia_de(tabla) == Copia::Agregar;
                cambiadas += almacen
                    .aplicar_de_la_nube(cambios, inmutable)
                    .await
                    .map_err(|e| ErrorNube::Bajada(format!("{tabla}: {e}")))?;
            }
            mayor = mayor.max(ultima);
            if !completa {
                break;
            }
            desde = ultima + 1;
        }
        if hasta_ahora != Some(mayor) {
            almacen
                .anotar_bajado_hasta_en_el_diario(mayor)
                .await
                .map_err(|e| error(&e))?;
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

    /// Las anotaciones del diario de la nube desde `desde` (microsegundos, incluida), en orden,
    /// a lo sumo [`POR_LOTE`].
    async fn diario_desde(&self, desde: i64) -> Result<Vec<Anotacion>, surrealdb::Error> {
        let mut respuesta = self
            .db
            .query(
                "SELECT time::micros(en) AS micros, tabla, registro, tipo FROM cambio \
                 WHERE en >= time::from_micros($desde) ORDER BY en LIMIT $limite",
            )
            .bind(("desde", desde))
            .bind(("limite", i64::try_from(POR_LOTE).unwrap_or(i64::MAX)))
            .await?;
        let filas: Vec<Value> = respuesta.take(0)?;
        Ok(filas.into_iter().filter_map(Anotacion::de).collect())
    }

    /// Lo que cambió en cada fila anotada: de cada una sólo cuenta la última anotación. Un
    /// borrado se aplica como borrado; lo demás, con la fila tal como está ahora (si ya no
    /// existe o no es de esta unidad, no hay nada que aplicar).
    async fn cambios_de(
        &self,
        anotaciones: Vec<Anotacion>,
    ) -> Result<Vec<Cambio>, surrealdb::Error> {
        let mut ultimas: Vec<Anotacion> = Vec::new();
        for anotacion in anotaciones.into_iter().rev() {
            if !ultimas
                .iter()
                .any(|vista| vista.registro == anotacion.registro)
            {
                ultimas.push(anotacion);
            }
        }
        ultimas.reverse();
        let mut cambios = Vec::new();
        let mut vigentes = Vec::new();
        for anotacion in ultimas {
            if anotacion.tipo == "DELETE" {
                cambios.push(Cambio::Borrada(anotacion.registro));
            } else {
                vigentes.push(anotacion.registro);
            }
        }
        if !vigentes.is_empty() {
            let mut respuesta = self
                .db
                .query("SELECT * FROM $registros")
                .bind(("registros", vigentes))
                .await?;
            let filas: Vec<Value> = respuesta.take(0)?;
            cambios.extend(filas.into_iter().map(Cambio::Guardada));
        }
        Ok(cambios)
    }
}

/// Una anotación del diario: qué fila cambió y cómo.
struct Anotacion {
    micros: i64,
    registro: RecordId,
    tipo: String,
}

impl Anotacion {
    fn de(fila: Value) -> Option<Self> {
        let Value::Object(campos) = fila else {
            return None;
        };
        let (
            Some(Value::Number(Number::Int(micros))),
            Some(Value::RecordId(registro)),
            Some(Value::String(tipo)),
        ) = (
            campos.get("micros"),
            campos.get("registro"),
            campos.get("tipo"),
        )
        else {
            return None;
        };
        Some(Self {
            micros: *micros,
            registro: registro.clone(),
            tipo: tipo.clone(),
        })
    }
}

/// Reparte los cambios de una tanda por tabla, en el orden de `tablas` (los catálogos antes de
/// lo que los usa), y de cada fila deja sólo el último. Las tablas sin cambios no aparecen.
pub fn por_tabla(
    cambios: Vec<Cambio>,
    tablas: &[&'static str],
) -> Vec<(&'static str, Vec<Cambio>)> {
    tablas
        .iter()
        .filter_map(|&tabla| {
            let propios: Vec<Cambio> = cambios
                .iter()
                .filter(|cambio| tabla_de(cambio) == Some(tabla))
                .cloned()
                .collect();
            (!propios.is_empty()).then(|| (tabla, colapsar(propios)))
        })
        .collect()
}

/// Deja sólo el último cambio de cada fila, en el orden en que quedaron.
///
/// El registro de cambios guarda cada versión por la que pasó una fila. Aplicarlas todas, una
/// tras otra, escribe versiones intermedias que ya no son las actuales; cada escritura deja una
/// entrada nueva en el registro del otro lado, y eso vuelve a viajar: el equipo y la nube se
/// corrigen sin parar. Con sólo la última, lo que ya está igual no se escribe y el vaivén termina.
pub fn colapsar(cambios: Vec<Cambio>) -> Vec<Cambio> {
    let mut vistos = std::collections::HashSet::new();
    let mut ultimos: Vec<Cambio> = cambios
        .into_iter()
        .rev()
        .filter(|cambio| clave_de(cambio).is_none_or(|clave| vistos.insert(clave)))
        .collect();
    ultimos.reverse();
    ultimos
}

/// Qué fila toca un cambio (sin ella, el cambio se conserva tal cual).
fn clave_de(cambio: &Cambio) -> Option<String> {
    match cambio {
        Cambio::Guardada(Value::Object(campos)) => match campos.get("id") {
            Some(Value::RecordId(id)) => Some(format!("{id:?}")),
            _ => None,
        },
        Cambio::Guardada(_) => None,
        Cambio::Borrada(id) => Some(format!("{id:?}")),
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
            "un cambio posterior llega por el diario"
        );
    }

    /// Cuántas anotaciones lleva el diario de la nube de una tabla.
    async fn entradas_en_la_nube(nube: &Nube, tabla: &str) -> usize {
        let mut respuesta = nube
            .db
            .query("SELECT VALUE id FROM cambio WHERE tabla = $tabla")
            .bind(("tabla", tabla.to_owned()))
            .await
            .unwrap();
        let ids: Vec<RecordId> = respuesta.take(0).unwrap();
        ids.len()
    }

    /// Una fila que cambió dos veces en la nube deja dos entradas en su registro. Al
    /// sincronizar, el equipo debe quedarse con la versión final y **dejar de escribir**: si
    /// aplicara la intermedia, cada vuelta escribiría en los dos lados y no pararía nunca.
    #[tokio::test]
    async fn una_fila_con_dos_versiones_en_el_historial_no_hace_rebotar_a_nadie() {
        let nube = nube().await;
        let equipo = AlmacenSurreal::en_memoria().await.unwrap();
        for nombre in ["ACME", "ACME SOCIEDAD"] {
            nube.db
                .query(
                    "UPSERT type::record('empresa', <uuid>'0199c7a2-0000-7000-8000-0000000000bb') \
                     CONTENT { nombre: $nombre, nombre_busqueda: string::lowercase($nombre) };",
                )
                .bind(("nombre", nombre.to_owned()))
                .await
                .unwrap()
                .check()
                .unwrap();
        }

        // La primera vuelta baja; la segunda devuelve una vez lo que el equipo acaba de escribir
        // (el eco inicial). A partir de ahí, nadie escribe más.
        vuelta(&nube, &equipo).await;
        vuelta(&nube, &equipo).await;
        let entradas = entradas_en_la_nube(&nube, "empresa").await;
        for _ in 0..4 {
            assert_eq!(
                vuelta(&nube, &equipo).await,
                0,
                "pasado el eco inicial, ya no baja nada"
            );
        }
        assert_eq!(
            entradas_en_la_nube(&nube, "empresa").await,
            entradas,
            "y el equipo no vuelve a escribir en la nube"
        );
    }

    /// El otro caso real: una presencia que se creó y se borró (alguien entró y salió). El
    /// último cambio es un borrado y tampoco debe rebotar.
    #[tokio::test]
    async fn una_presencia_creada_y_borrada_en_la_nube_no_hace_rebotar_a_nadie() {
        let nube = nube().await;
        let equipo = AlmacenSurreal::en_memoria().await.unwrap();
        nube.db
            .query(
                "CREATE type::record('presencia', '603500432') CONTENT                  { unidad: 'BRISAS', via: 'CONTRATISTA', desde: time::now() };                  DELETE type::record('presencia', '603500432');",
            )
            .await
            .unwrap()
            .check()
            .unwrap();

        vuelta(&nube, &equipo).await;
        vuelta(&nube, &equipo).await;
        let entradas = entradas_en_la_nube(&nube, "presencia").await;
        for _ in 0..4 {
            assert_eq!(vuelta(&nube, &equipo).await, 0, "nada que bajar");
        }
        assert_eq!(
            entradas_en_la_nube(&nube, "presencia").await,
            entradas,
            "y nada que subir"
        );
        assert!(!adentro(&equipo, "603500432").await, "nadie quedó adentro");
    }

    fn hecho_de_prueba(equipo: &AlmacenSurreal, numero: u128) -> limen_dominio::hecho::Hecho {
        use limen_dominio::hecho::{CalidadHora, Hecho, HechoId, Salida, Suceso};
        use limen_dominio::movimiento::Marca;
        use limen_dominio::operador::OperadorId;
        use uuid::Uuid;

        Hecho::restaurar(
            HechoId::desde_uuid(Uuid::from_u128(numero)),
            Suceso::Salida(Salida {
                via: Via::Contratista,
                registro: Uuid::from_u128(5),
                marca: Marca {
                    en: en(),
                    operador: OperadorId::desde_uuid(Uuid::from_u128(9)),
                },
            }),
            equipo.origen().clone(),
            CalidadHora {
                confiable: true,
                hora_equipo: en(),
            },
        )
    }

    async fn hechos_en_la_nube(nube: &Nube) -> usize {
        let mut respuesta = nube.db.query("SELECT VALUE id FROM hecho").await.unwrap();
        let ids: Vec<RecordId> = respuesta.take(0).unwrap();
        ids.len()
    }

    /// El caso de la instalación nueva: el equipo baja los hechos que ya había en la nube y
    /// después registra uno propio. Ese hecho nuevo tiene que llegar a la nube.
    #[tokio::test]
    async fn un_hecho_nuevo_sube_despues_de_bajar_los_que_ya_habia() {
        use limen_aplicacion::puertos::{FabricaUnidadDeTrabajo, RegistroHechos, UnidadDeTrabajo};

        let nube = nube().await;
        let otro = AlmacenSurreal::en_memoria().await.unwrap();
        let mut uow = otro.nueva();
        uow.hechos().anotar(hecho_de_prueba(&otro, 1));
        uow.confirmar().await.unwrap();
        vuelta(&nube, &otro).await;
        assert_eq!(hechos_en_la_nube(&nube).await, 1, "el otro equipo lo subió");

        let equipo = AlmacenSurreal::en_memoria().await.unwrap();
        vuelta(&nube, &equipo).await;
        vuelta(&nube, &equipo).await;
        let mut uow = equipo.nueva();
        uow.hechos().anotar(hecho_de_prueba(&equipo, 2));
        uow.confirmar().await.unwrap();
        vuelta(&nube, &equipo).await;

        assert_eq!(
            hechos_en_la_nube(&nube).await,
            2,
            "el hecho que registró el equipo nuevo llegó a la nube"
        );
    }

    /// `SurrealDB` aplica el `LIMIT` de `SHOW CHANGES` antes de filtrar por tabla: si entre el
    /// cursor de una tabla y su entrada nueva hay muchos cambios de otras tablas (la descarga
    /// de una instalación nueva escribe miles), la consulta con límite no ve nada de esa tabla
    /// y el cursor no avanza nunca. Pasaba con el hecho de la entrada de un ingreso.
    #[tokio::test]
    async fn un_hecho_nuevo_sube_aunque_haya_muchos_cambios_de_otras_tablas_en_medio() {
        use limen_aplicacion::puertos::{
            FabricaUnidadDeTrabajo, RegistroHechos, RepositorioGafetes, UnidadDeTrabajo,
        };
        use limen_dominio::gafete::{Gafete, NumeroGafete, TipoGafete};

        let nube = nube().await;
        let equipo = AlmacenSurreal::en_memoria().await.unwrap();
        let mut uow = equipo.nueva();
        uow.hechos().anotar(hecho_de_prueba(&equipo, 1));
        uow.confirmar().await.unwrap();
        vuelta(&nube, &equipo).await;
        assert_eq!(hechos_en_la_nube(&nube).await, 1);

        // Muchas transacciones de otra tabla entre el cursor de `hecho` y su próxima entrada.
        for numero in 1..=250_u32 {
            let mut uow = equipo.nueva();
            let numero = NumeroGafete::nuevo(numero).unwrap();
            uow.gafetes()
                .agregar(&Gafete::registrar(TipoGafete::Contratista, numero, false).unwrap());
            uow.confirmar().await.unwrap();
        }
        let mut uow = equipo.nueva();
        uow.hechos().anotar(hecho_de_prueba(&equipo, 2));
        uow.confirmar().await.unwrap();

        vuelta(&nube, &equipo).await;

        assert_eq!(
            hechos_en_la_nube(&nube).await,
            2,
            "el hecho nuevo llegó a la nube aunque 250 transacciones de gafetes lo separaran del cursor"
        );
    }

    // --- Un equipo con token limitado a su unidad ---

    const ACCESO_DE_PRUEBA: &str = "
        DEFINE ACCESS OVERWRITE equipo ON DATABASE TYPE RECORD WITH JWT ALGORITHM HS512
            KEY 'clave-de-prueba-que-solo-conocen-el-worker-y-la-base'
            AUTHENTICATE { RETURN type::record('equipo', $token.equipo) }
            DURATION FOR SESSION 1h;
    ";

    /// El token que en producción emite el Worker del panel, para la unidad de las pruebas.
    fn token_de_pruebas() -> String {
        use base64::Engine as _;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use hmac::{Hmac, Mac};
        use sha2::Sha512;

        let cabecera = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS512","typ":"JWT"}"#);
        let datos = format!(
            r#"{{"ns":"limen","db":"principal","ac":"equipo","unidad":"{}","equipo":"equipo-1","iat":1700000000,"exp":4000000000}}"#,
            limen_infra_surreal::UNIDAD_DE_PRUEBAS
        );
        let cuerpo = format!("{cabecera}.{}", URL_SAFE_NO_PAD.encode(datos));
        let mut firma =
            Hmac::<Sha512>::new_from_slice(b"clave-de-prueba-que-solo-conocen-el-worker-y-la-base")
                .unwrap();
        firma.update(cuerpo.as_bytes());
        format!(
            "{cuerpo}.{}",
            URL_SAFE_NO_PAD.encode(firma.finalize().into_bytes())
        )
    }

    /// La nube tal como la ve un equipo con token (otra sesión sobre la misma base) y la nube del
    /// administrador, que siembra y revisa sin pasar por los permisos.
    async fn nube_con_token_de_un_equipo() -> (Nube, Nube) {
        let administrador = nube().await;
        administrador
            .db
            .query(ACCESO_DE_PRUEBA)
            .await
            .unwrap()
            .check()
            .unwrap();
        let sesion = administrador.db.clone();
        sesion.authenticate(token_de_pruebas()).await.unwrap();
        (administrador, Nube { db: sesion })
    }

    async fn cuantas_presencias(nube: &Nube) -> usize {
        let mut respuesta = nube
            .db
            .query("SELECT VALUE id FROM presencia")
            .await
            .unwrap();
        let ids: Vec<RecordId> = respuesta.take(0).unwrap();
        ids.len()
    }

    /// El equipo entra con su token (no con una clave de administrador): sube lo suyo, baja lo
    /// global y lo de su unidad, y de otra unidad no recibe nada.
    #[tokio::test]
    async fn un_equipo_con_token_sube_y_baja_solo_lo_de_su_unidad() {
        let (administrador, nube_del_equipo) = nube_con_token_de_un_equipo().await;
        administrador
            .db
            .query(
                "CREATE presencia:`222222222` CONTENT { unidad: 'OTRA', via: 'PROVEEDOR', \
                 desde: d'2026-10-09T08:00:00Z' }; \
                 CREATE empresa:u'0199c7a2-0000-7000-8000-0000000000cc' \
                 CONTENT { nombre: 'ACME', nombre_busqueda: 'acme' };",
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        let equipo = AlmacenSurreal::en_memoria().await.unwrap();
        entra(&equipo, "111111111").await;

        nube_del_equipo.subir_cambios(&equipo).await.unwrap();
        nube_del_equipo.bajar_cambios(&equipo).await.unwrap();

        assert_eq!(
            cuantas_presencias(&administrador).await,
            2,
            "lo del equipo llegó a la nube, junto a lo de la otra unidad"
        );
        assert!(
            !adentro(&equipo, "222222222").await,
            "lo de otra unidad no baja"
        );
        assert_eq!(
            equipo.filas("empresa").await.unwrap().len(),
            1,
            "los catálogos globales sí bajan"
        );
        assert!(adentro(&equipo, "111111111").await);
    }

    /// Una persona que ya está adentro en otra unidad (regla A7): la nube rechaza el segundo
    /// registro aunque el equipo no pueda verla. Eso no debe frenar lo demás: se anota y se sigue.
    #[tokio::test]
    async fn un_choque_con_otra_unidad_no_frena_el_resto_de_la_subida() {
        let (administrador, nube_del_equipo) = nube_con_token_de_un_equipo().await;
        administrador
            .db
            .query(
                "CREATE presencia:`111111111` CONTENT { unidad: 'OTRA', via: 'PROVEEDOR', \
                 desde: d'2026-10-09T08:00:00Z' }",
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        let equipo = AlmacenSurreal::en_memoria().await.unwrap();
        entra(&equipo, "111111111").await;
        entra(&equipo, "333333333").await;

        nube_del_equipo
            .subir_cambios(&equipo)
            .await
            .expect("el choque no debe cortar la subida");

        assert_eq!(
            cuantas_presencias(&administrador).await,
            2,
            "la de la otra unidad sigue siendo suya y la otra persona sí subió"
        );
        let mut respuesta = administrador
            .db
            .query("SELECT VALUE unidad FROM presencia:`111111111`")
            .await
            .unwrap();
        let unidades: Vec<String> = respuesta.take(0).unwrap();
        assert_eq!(
            unidades,
            vec!["OTRA".to_owned()],
            "no se la quitó a la otra unidad"
        );
    }

    /// El aviso en tiempo real también funciona con el token de un equipo: le llega lo de su
    /// unidad y lo global, y de otra unidad no le avisa nada.
    #[tokio::test]
    async fn el_aviso_en_vivo_funciona_con_el_token_de_un_equipo() {
        use std::sync::Arc;
        use std::time::Duration;

        use tokio::sync::Notify;

        let (administrador, nube_del_equipo) = nube_con_token_de_un_equipo().await;
        let avisos = Arc::new(Notify::new());
        let _escucha = nube_del_equipo
            .escuchar(limen_infra_surreal::UNIDAD_DE_PRUEBAS, Arc::clone(&avisos))
            .await
            .unwrap();
        let avisa = |avisos: Arc<Notify>| async move {
            tokio::time::timeout(Duration::from_secs(2), avisos.notified())
                .await
                .is_ok()
        };

        administrador
            .db
            .query(
                "CREATE presencia:`555555555` CONTENT { unidad: 'OTRA', via: 'PROVEEDOR', \
                 desde: d'2026-10-09T08:00:00Z' }",
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        assert!(
            !avisa(Arc::clone(&avisos)).await,
            "lo de otra unidad no le avisa nada"
        );

        administrador
            .db
            .query(
                "CREATE presencia:`444444444` CONTENT { unidad: 'PRUEBAS', via: 'PROVEEDOR', \
                 desde: d'2026-10-09T08:00:00Z' }",
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        assert!(avisa(Arc::clone(&avisos)).await, "lo de su unidad sí");

        administrador
            .db
            .query("CREATE empresa:dos CONTENT { nombre: 'ACME', nombre_busqueda: 'acme' }")
            .await
            .unwrap()
            .check()
            .unwrap();
        assert!(avisa(avisos).await, "y lo global, que es de todos");
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
