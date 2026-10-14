//! Conexión a la base embebida y aplicación del esquema.

use std::path::Path;
use std::sync::Arc;

use limen_aplicacion::puertos::{ErrorPersistencia, RegistroAuditado};
use limen_dominio::origen::{EquipoId, Origen, UnidadOperativaId};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::types::Value;
use tokio::sync::Notify;
use uuid::Uuid;

use crate::error::tecnica;
use crate::registros::AuditoriaRegistro;

/// El esquema completo, declarativo (`OVERWRITE`): se vuelve a aplicar entero
/// en cada arranque. Es el mismo en los equipos y en la nube.
pub const ESQUEMA: &str = include_str!("esquema/esquema.surql");

/// Lo que sólo existe en la base de un equipo, no en la nube: la bandeja de salida y sus eventos.
const ESQUEMA_DEL_EQUIPO: &str = include_str!("esquema/equipo.surql");
/// El namespace y la base de Limen, iguales en los equipos y en la nube.
pub const NAMESPACE: &str = "limen";
pub const BASE: &str = "principal";

/// La unidad operativa de las bases en memoria (pruebas).
pub const UNIDAD_DE_PRUEBAS: &str = "PRUEBAS";

/// Convierte lo guardado antes de las unidades operativas (M4). Ver el
/// archivo: sólo toca lo que todavía no tiene unidad.
const SIN_UNIDAD: &str = include_str!("esquema/sin_unidad.surql");

/// La base de datos embebida de un equipo. Clonarla comparte la misma
/// conexión: es lo que reciben la composición y la fábrica de Unit of Work.
///
/// Conoce el origen del equipo (M4): la base local sólo guarda lo de su
/// unidad, que va en las claves de los gafetes y en el campo `unidad` de
/// cada registro de la unidad.
#[derive(Debug, Clone)]
pub struct AlmacenSurreal {
    db: Surreal<Db>,
    origen: Origen,
    /// Se avisa cada vez que una Unit of Work confirma algo: la
    /// sincronización con la nube lo sube en el momento.
    confirmaciones: Arc<Notify>,
}

impl AlmacenSurreal {
    /// Base en memoria, nueva y vacía, con el esquema aplicado, de la unidad
    /// [`UNIDAD_DE_PRUEBAS`]. Para pruebas.
    pub async fn en_memoria() -> Result<Self, ErrorPersistencia> {
        let db = Surreal::new::<Mem>(()).await.map_err(tecnica)?;
        let origen = Origen {
            unidad: UnidadOperativaId::nueva(UNIDAD_DE_PRUEBAS)
                .map_err(|e| ErrorPersistencia::Tecnica(e.to_string()))?,
            equipo: EquipoId::desde_uuid(Uuid::nil()),
        };
        Self::preparar(db, origen).await
    }

    /// Base en disco (`SurrealKV`) en `ruta`, del equipo `origen`, con el
    /// esquema aplicado. Si no existe, se crea; si es de antes de las
    /// unidades operativas, se convierte (`sin_unidad.surql`).
    ///
    /// Una sola apertura por proceso: el archivo queda tomado mientras viva
    /// algún clon del almacén, y al soltar el último el motor lo libera en
    /// segundo plano (no hay un cierre que se pueda esperar). Abrir la misma
    /// ruta dos veces a la vez falla con un error técnico.
    pub async fn en_disco(ruta: &Path, origen: Origen) -> Result<Self, ErrorPersistencia> {
        let ruta = ruta.to_str().ok_or_else(|| {
            ErrorPersistencia::Tecnica(format!("ruta no válida: {}", ruta.display()))
        })?;
        let db = Surreal::new::<SurrealKv>(ruta).await.map_err(tecnica)?;
        Self::preparar(db, origen).await
    }

    async fn preparar(db: Surreal<Db>, origen: Origen) -> Result<Self, ErrorPersistencia> {
        db.use_ns(NAMESPACE).use_db(BASE).await.map_err(tecnica)?;
        db.query(ESQUEMA)
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map_err(tecnica)?;
        db.query(ESQUEMA_DEL_EQUIPO)
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map_err(tecnica)?;
        let almacen = Self {
            db,
            origen,
            confirmaciones: Arc::new(Notify::new()),
        };
        almacen.convertir_lo_que_no_tiene_unidad().await?;
        Ok(almacen)
    }

    async fn convertir_lo_que_no_tiene_unidad(&self) -> Result<(), ErrorPersistencia> {
        self.db
            .query(SIN_UNIDAD)
            .bind(("unidad", self.origen.unidad.to_string()))
            .await
            .and_then(surrealdb::IndexedResults::check)
            .map(drop)
            .map_err(tecnica)
    }

    /// Espera hasta que la app confirme algo en esta base. Si ya confirmó
    /// algo desde la última espera, vuelve en seguida: no se pierde ningún
    /// aviso.
    pub async fn esperar_confirmacion(&self) {
        self.confirmaciones.notified().await;
    }

    pub(crate) fn avisar_confirmacion(&self) {
        self.confirmaciones.notify_one();
    }

    /// La unidad operativa de este equipo.
    pub const fn unidad(&self) -> &UnidadOperativaId {
        &self.origen.unidad
    }

    /// Dónde está este equipo. Es también el origen de los hechos guardados
    /// antes de que los hechos lo llevaran: se registraron aquí.
    pub const fn origen(&self) -> &Origen {
        &self.origen
    }

    /// Todos los registros de una tabla, tal como están guardados (con su
    /// `id`). Es para copiar la base del equipo a la nube, que tiene el mismo
    /// esquema: no pasa por el dominio porque no decide nada, sólo copia.
    pub async fn filas(&self, tabla: &str) -> Result<Vec<Value>, ErrorPersistencia> {
        let mut respuesta = self
            .db
            .query("SELECT * FROM type::table($tabla) ORDER BY id")
            .bind(("tabla", tabla.to_owned()))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
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
    use crate::registros::TABLAS_DE_LA_UNIDAD;
    use chrono::{DateTime, Utc};
    use limen_aplicacion::puertos::{
        AccionAuditada, Consultas, EntradaAuditoria, FabricaUnidadDeTrabajo, RegistroAuditoria,
        RegistroHechos, UnidadDeTrabajo,
    };
    use limen_aplicacion::puertos::{RepositorioGafetes, RepositorioPresencias};
    use limen_dominio::auditoria::CambioCampo;
    use limen_dominio::cedula::Cedula;
    use limen_dominio::empresa::EmpresaId;
    use limen_dominio::gafete::{Gafete, NumeroGafete, TipoGafete};
    use limen_dominio::hecho::{CalidadHora, Hecho, HechoId, Salida, Suceso};
    use limen_dominio::movimiento::Marca;
    use limen_dominio::operador::OperadorId;
    use limen_dominio::presencia::Identidad;
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
            Origen {
                unidad: almacen.unidad().clone(),
                equipo: EquipoId::desde_uuid(Uuid::from_u128(6)),
            },
            CalidadHora {
                confiable: true,
                hora_equipo: en(),
            },
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
    async fn lo_de_la_unidad_lleva_su_unidad_y_el_gafete_la_lleva_en_la_clave() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let numero = NumeroGafete::nuevo(25).unwrap();
        let gafete = Gafete::registrar(TipoGafete::Contratista, numero, false).unwrap();
        let cedula = Cedula::normalizar("111111111").unwrap();
        let mut uow = almacen.nueva();
        uow.gafetes().agregar(&gafete);
        uow.gafetes()
            .anotar_prestamo(TipoGafete::Contratista, numero, en());
        uow.presencias()
            .anotar_entrada(&Identidad::from(&cedula), Via::Contratista, en());
        uow.confirmar().await.unwrap();

        let mut respuesta = almacen
            .db()
            .query(
                "SELECT VALUE unidad FROM gafete:`PRUEBAS-CONTRATISTA-25`; \
                 SELECT VALUE unidad FROM prestamo_gafete:`PRUEBAS-CONTRATISTA-25`; \
                 SELECT VALUE unidad FROM presencia;",
            )
            .await
            .unwrap();
        for (i, que) in ["el gafete", "su préstamo", "la presencia"]
            .into_iter()
            .enumerate()
        {
            let unidades: Vec<String> = respuesta.take(i).unwrap();
            assert_eq!(unidades, ["PRUEBAS"], "{que} lleva la unidad");
        }
        assert!(
            almacen
                .nueva()
                .gafetes()
                .prestado(TipoGafete::Contratista, numero)
                .await
                .unwrap(),
            "y se encuentra por su clave con la unidad"
        );
    }

    /// Una base como las de antes de las unidades operativas: los campos
    /// `unidad` y el origen de los hechos no existían, y los gafetes tenían
    /// la clave `TIPO-NÚMERO`.
    async fn base_de_antes() -> AlmacenSurreal {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let mut quitar: Vec<String> = TABLAS_DE_LA_UNIDAD
            .iter()
            .map(|tabla| format!("REMOVE FIELD unidad ON {tabla};"))
            .collect();
        quitar.push("REMOVE FIELD unidad ON hecho; REMOVE FIELD equipo ON hecho;".to_owned());
        let sentencias = quitar.join(" ")
            + " CREATE gafete:`CONTRATISTA-25` CONTENT { tipo: 'CONTRATISTA', numero: 25, estado: 'DISPONIBLE' };
                CREATE gafete:`VISITA-3` CONTENT { tipo: 'VISITA', numero: 3, estado: 'PERDIDO', portador_cedula: '111111111' };
                CREATE prestamo_gafete:`CONTRATISTA-25` CONTENT { desde: d'2026-10-09T08:00:00Z' };
                CREATE presencia:`111111111` CONTENT { via: 'CONTRATISTA', desde: d'2026-10-09T08:00:00Z' };
                CREATE hecho:u'00000000-0000-0000-0000-000000000001' CONTENT {
                    tipo: 'SALIDA', via: 'CONTRATISTA', registro: u'00000000-0000-0000-0000-000000000005',
                    en: d'2026-10-09T17:00:00Z', operador: u'00000000-0000-0000-0000-000000000009',
                    hora_confiable: true, hora_equipo: d'2026-10-09T17:00:00Z' };";
        let errores = errores_de(&almacen, &sentencias).await;
        assert_eq!(errores, Vec::<String>::new(), "la base de antes se arma");
        almacen
            .db()
            .query(ESQUEMA)
            .await
            .and_then(surrealdb::IndexedResults::check)
            .unwrap();
        almacen
    }

    #[tokio::test]
    async fn lo_de_antes_de_las_unidades_se_convierte_sin_perder_nada() {
        let almacen = base_de_antes().await;

        almacen.convertir_lo_que_no_tiene_unidad().await.unwrap();

        let mut respuesta = almacen
            .db()
            .query(
                "SELECT VALUE record::id(id) FROM gafete ORDER BY id; \
                 SELECT VALUE unidad FROM gafete; \
                 SELECT VALUE portador_cedula FROM ONLY gafete:`PRUEBAS-VISITA-3`; \
                 SELECT VALUE record::id(id) FROM prestamo_gafete; \
                 SELECT VALUE unidad FROM presencia;",
            )
            .await
            .unwrap();
        let claves: Vec<String> = respuesta.take(0).unwrap();
        assert_eq!(
            claves,
            ["PRUEBAS-CONTRATISTA-25", "PRUEBAS-VISITA-3"],
            "los gafetes pasan a la clave con la unidad, y no queda ninguno con la vieja"
        );
        let unidades: Vec<String> = respuesta.take(1).unwrap();
        assert_eq!(unidades, ["PRUEBAS", "PRUEBAS"], "y llevan su unidad");
        let portador: Option<String> = respuesta.take(2).unwrap();
        assert_eq!(
            portador.as_deref(),
            Some("111111111"),
            "con todos sus datos"
        );
        let prestamos: Vec<String> = respuesta.take(3).unwrap();
        assert_eq!(prestamos, ["PRUEBAS-CONTRATISTA-25"], "el préstamo también");
        let presencia: Vec<String> = respuesta.take(4).unwrap();
        assert_eq!(presencia, ["PRUEBAS"], "lo demás recibe el campo `unidad`");

        let numero = NumeroGafete::nuevo(25).unwrap();
        assert!(
            almacen
                .nueva()
                .gafetes()
                .prestado(TipoGafete::Contratista, numero)
                .await
                .unwrap(),
            "la app encuentra el préstamo convertido"
        );
        let hechos = almacen.hechos_de(Uuid::from_u128(5)).await.unwrap();
        assert_eq!(
            hechos.first().map(Hecho::origen),
            Some(almacen.origen()),
            "un hecho sin origen se lee como registrado en este equipo"
        );
    }

    #[tokio::test]
    async fn convertir_otra_vez_no_cambia_nada() {
        let almacen = base_de_antes().await;
        almacen.convertir_lo_que_no_tiene_unidad().await.unwrap();
        let antes = cuantos(&almacen, "gafete").await;

        almacen.convertir_lo_que_no_tiene_unidad().await.unwrap();

        assert_eq!(cuantos(&almacen, "gafete").await, antes, "no duplica nada");
        assert_eq!(antes, 2, "siguen los dos gafetes");
    }

    #[tokio::test]
    async fn avisa_cuando_se_confirma_algo_y_no_antes() {
        let almacen = AlmacenSurreal::en_memoria().await.unwrap();
        let espera = std::time::Duration::from_millis(200);
        assert!(
            tokio::time::timeout(espera, almacen.esperar_confirmacion())
                .await
                .is_err(),
            "sin confirmar nada no hay aviso"
        );

        let mut uow = almacen.nueva();
        uow.presencias().anotar_entrada(
            &Identidad::from(&Cedula::normalizar("111111111").unwrap()),
            Via::Contratista,
            en(),
        );
        uow.confirmar().await.unwrap();

        assert!(
            tokio::time::timeout(espera, almacen.esperar_confirmacion())
                .await
                .is_ok(),
            "lo confirmado avisa, aunque nadie estuviera esperando en ese momento"
        );
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
                 operador: u'00000000-0000-0000-0000-000000000009', \
                 unidad: 'PRUEBAS', equipo: u'00000000-0000-0000-0000-000000000006', \
                 hora_confiable: true, hora_equipo: d'2026-10-09T17:00:00Z' }};"
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
            "CREATE presencia:x CONTENT { unidad: 'PRUEBAS', via: 'CONTRATISTA', desde: d'2026-10-09T08:00:00Z' }; \
             DELETE presencia:x;",
        )
        .await;
        assert_eq!(
            errores,
            Vec::<String>::new(),
            "la presencia se borra sin problema"
        );
    }
}
