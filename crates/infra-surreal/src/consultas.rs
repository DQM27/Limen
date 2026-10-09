//! Consultas de lectura sobre `SurrealDB` (puerto `Consultas`).

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use limen_aplicacion::puertos::{
    AccionAuditada, CambioHistorial, Consultas, EntradaHistorial, ErrorPersistencia,
    IngresoAbierto, PersonaAdentro, RegistroAuditado, ResumenGafete,
};
use limen_dominio::busqueda::{
    Buscable, Criterio, MINIMO_DIGITOS_PARA_SUBCADENA, Nivel, cuantos_hasta, relevantes,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::Empresa;
use limen_dominio::empresa_proveedora::EmpresaProveedora;
use limen_dominio::gafete::{Gafete, TipoGafete};
use limen_dominio::ingreso_contratista::IngresoId;
use limen_dominio::ingreso_correo::IngresoCorreoId;
use limen_dominio::ingreso_proveedor::IngresoProveedorId;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::personal_kof::PersonalKof;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};

use crate::almacen::AlmacenSurreal;
use crate::error::{dato_corrupto, tecnica};
use crate::registros::clave_gafete;
use crate::registros::{
    ContratistaLeido, EmpresaLeida, EmpresaProveedoraLeida, GafeteDatos, PersonalKofLeido,
    TABLA_AUDITORIA, TABLA_INGRESO_CONTRATISTA, TABLA_INGRESO_CORREO, TABLA_INGRESO_PROVEEDOR,
    medio_de, numero_de, uuid_de,
};

/// Sólo el nombre plegado de una fila: lo que necesita la búsqueda
/// aproximada para decidir cuáles pedir completas.
#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
struct FilaNombre {
    id: RecordId,
    nombre_busqueda: String,
}

/// Una fila de "quién está adentro", igual para las tres vías: cada consulta
/// pone en `nombre` y `procedencia` lo que corresponde a su tabla.
#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
struct FilaAdentro {
    id: RecordId,
    cedula: String,
    nombre: Option<String>,
    procedencia: Option<String>,
    placa: Option<String>,
    gafete: Option<i64>,
    entrada_en: DateTime<Utc>,
}

const ADENTRO_CONTRATISTAS: &str = "SELECT id, cedula, contratista.nombre AS nombre, \
    contratista.empresa.nombre AS procedencia, placa, gafete, entrada_en \
    FROM ingreso_contratista WHERE salida_en = NONE";
const ADENTRO_PROVEEDORES: &str = "SELECT id, cedula, nombre, empresa.nombre AS procedencia, \
    placa, gafete, entrada_en FROM ingreso_proveedor WHERE salida_en = NONE";
const ADENTRO_CORREO: &str = "SELECT id, cedula, nombre, motivo AS procedencia, \
    placa, gafete, entrada_en FROM ingreso_correo WHERE salida_en = NONE";

impl FilaAdentro {
    /// `None` si la fila no tiene nombre (el contratista ya no existe): no
    /// debería pasar, porque nada se borra.
    fn a_persona(
        self,
        tabla: &str,
        ingreso: fn(uuid::Uuid) -> IngresoAbierto,
    ) -> Result<Option<PersonaAdentro>, ErrorPersistencia> {
        let corrupto = |detalle: String| dato_corrupto(tabla, detalle);
        let Some(nombre) = self.nombre else {
            return Ok(None);
        };
        Ok(Some(PersonaAdentro {
            ingreso: ingreso(uuid_de(&self.id, tabla)?),
            cedula: Cedula::normalizar(&self.cedula).map_err(|e| corrupto(e.to_string()))?,
            nombre: NombrePersona::nuevo(&nombre).map_err(|e| corrupto(e.to_string()))?,
            procedencia: self.procedencia.unwrap_or_default(),
            medio: medio_de(self.placa, tabla)?,
            gafete: self.gafete.map(|n| numero_de(n, tabla)).transpose()?,
            desde: self.entrada_en,
        }))
    }
}

/// Una etapa de la búsqueda: una condición `WHERE` y los textos que se
/// enlazan a ella. El texto del operador nunca se concatena a la consulta:
/// sólo viaja en parámetros (`$p0`, `$p1`…). Lo único que se escribe dentro
/// de la consulta son números calculados acá (largos y tolerancias).
struct Etapa {
    consulta: Consulta,
    /// Hasta qué nivel garantiza esta etapa traer **todos** los que
    /// coinciden. Si ya hay suficientes resultados de ese nivel o mejor, las
    /// etapas siguientes (más caras) no pueden cambiar el resultado.
    completa_hasta: Nivel,
}

/// Cómo una etapa trae sus candidatos.
enum Consulta {
    /// Una condición `WHERE` con sus parámetros enlazados.
    Donde {
        donde: String,
        enlaces: Vec<(String, String)>,
    },
    /// Los errores de tecleo se buscan en Rust con la regla del dominio:
    /// la base sólo entrega `(id, nombre)` de cada fila y después se piden
    /// completas las que coinciden. Las funciones de distancia de la base
    /// dentro de una consulta resultaron unas 100 veces más lentas.
    Aproximada,
}

/// Las etapas de una búsqueda, de la más barata a la más cara.
///
/// - **Nombre**: primero el índice de texto (cada palabra escrita es el
///   comienzo de una palabra del nombre); si faltan resultados, lo que está
///   dentro del nombre; si todavía faltan, lo parecido (errores de tecleo).
/// - **Cédula o código**: primero el rango del índice (empieza igual); si
///   faltan resultados, lo que la contiene.
fn etapas(criterio: &Criterio, campo_identificacion: &str) -> Vec<Etapa> {
    match criterio {
        Criterio::Vacio => Vec::new(),
        Criterio::Identificacion(digitos) => {
            let mut etapas = vec![Etapa {
                consulta: Consulta::Donde {
                    donde: format!(
                        "{campo_identificacion} >= $p0 AND {campo_identificacion} < $p1"
                    ),
                    enlaces: vec![
                        ("p0".to_owned(), digitos.clone()),
                        ("p1".to_owned(), siguiente(digitos)),
                    ],
                },
                completa_hasta: Nivel::PrefijoDeIdentificacion,
            }];
            if digitos.len() >= MINIMO_DIGITOS_PARA_SUBCADENA {
                etapas.push(Etapa {
                    consulta: Consulta::Donde {
                        donde: format!("{campo_identificacion} CONTAINS $p0"),
                        enlaces: vec![("p0".to_owned(), digitos.clone())],
                    },
                    completa_hasta: Nivel::SubcadenaDeIdentificacion,
                });
            }
            etapas
        }
        Criterio::Nombre(palabras) if palabras.is_empty() => Vec::new(),
        Criterio::Nombre(palabras) => {
            let contienen = palabras
                .iter()
                .enumerate()
                .map(|(i, _)| format!("nombre_busqueda CONTAINS $p{i}"))
                .collect::<Vec<_>>()
                .join(" AND ");
            let enlaces = palabras
                .iter()
                .enumerate()
                .map(|(i, palabra)| (format!("p{i}"), palabra.clone()))
                .collect();
            vec![
                Etapa {
                    consulta: Consulta::Donde {
                        donde: "nombre_busqueda @AND@ $texto".to_owned(),
                        enlaces: vec![("texto".to_owned(), palabras.join(" "))],
                    },
                    completa_hasta: Nivel::PrefijoDePalabra,
                },
                Etapa {
                    consulta: Consulta::Donde {
                        donde: contienen,
                        enlaces,
                    },
                    completa_hasta: Nivel::Subcadena,
                },
                Etapa {
                    consulta: Consulta::Aproximada,
                    completa_hasta: Nivel::Aproximada,
                },
            ]
        }
    }
}

/// El primer texto que ya no empieza con `digitos` (para un rango por
/// prefijo): los mismos dígitos con el último aumentado en uno.
fn siguiente(digitos: &str) -> String {
    let mut caracteres: Vec<char> = digitos.chars().collect();
    if let Some(ultimo) = caracteres.last_mut() {
        *ultimo = char::from_u32(u32::from(*ultimo) + 1).unwrap_or(*ultimo);
    }
    caracteres.into_iter().collect()
}

impl AlmacenSurreal {
    async fn filas_adentro(&self, consulta: &str) -> Result<Vec<FilaAdentro>, ErrorPersistencia> {
        let mut respuesta = self.db().query(consulta).await.map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    /// Las filas cuyo nombre se parece a lo escrito (errores de tecleo). La
    /// regla es la del dominio; la base sólo entrega los nombres.
    async fn filas_parecidas<L: SurrealValue>(
        &self,
        tabla: &str,
        criterio: &Criterio,
    ) -> Result<Vec<L>, ErrorPersistencia> {
        let mut respuesta = self
            .db()
            .query(format!("SELECT id, nombre_busqueda FROM {tabla}"))
            .await
            .map_err(tecnica)?;
        let nombres: Vec<FilaNombre> = respuesta.take(0).map_err(tecnica)?;
        let parecidas: Vec<RecordId> = nombres
            .into_iter()
            .filter(|fila| criterio.nivel("", &fila.nombre_busqueda) == Some(Nivel::Aproximada))
            .map(|fila| fila.id)
            .collect();
        if parecidas.is_empty() {
            return Ok(Vec::new());
        }
        let mut respuesta = self
            .db()
            .query(format!("SELECT * FROM {tabla} WHERE id IN $ids"))
            .bind(("ids", parecidas))
            .await
            .map_err(tecnica)?;
        respuesta.take(0).map_err(tecnica)
    }

    /// Busca por etapas: cada una trae candidatos con una consulta; se
    /// detiene en cuanto ya hay `limite` resultados que ninguna etapa
    /// siguiente podría superar. El orden final lo decide el dominio.
    async fn buscar<L, T>(
        &self,
        tabla: &str,
        campo_identificacion: &str,
        criterio: &Criterio,
        limite: usize,
        convertir: fn(L) -> Result<T, ErrorPersistencia>,
    ) -> Result<Vec<T>, ErrorPersistencia>
    where
        L: SurrealValue,
        T: Buscable,
    {
        let mut candidatos: Vec<T> = Vec::new();
        let mut vistos: HashSet<String> = HashSet::new();
        for etapa in etapas(criterio, campo_identificacion) {
            let filas: Vec<L> = match etapa.consulta {
                Consulta::Donde { donde, enlaces } => {
                    let mut consulta = self
                        .db()
                        .query(format!("SELECT * FROM {tabla} WHERE {donde}"));
                    for enlace in enlaces {
                        consulta = consulta.bind(enlace);
                    }
                    let mut respuesta = consulta.await.map_err(tecnica)?;
                    respuesta.take(0).map_err(tecnica)?
                }
                Consulta::Aproximada => self.filas_parecidas(tabla, criterio).await?,
            };
            for fila in filas {
                let candidato = convertir(fila)?;
                if vistos.insert(candidato.identificacion()) {
                    candidatos.push(candidato);
                }
            }
            if cuantos_hasta(criterio, &candidatos, etapa.completa_hasta) >= limite {
                break;
            }
        }
        Ok(relevantes(criterio, candidatos, limite))
    }
}

impl Consultas for AlmacenSurreal {
    async fn quienes_estan_adentro(&self) -> Result<Vec<PersonaAdentro>, ErrorPersistencia> {
        let mut adentro = Vec::new();
        for fila in self.filas_adentro(ADENTRO_CONTRATISTAS).await? {
            adentro.extend(fila.a_persona(TABLA_INGRESO_CONTRATISTA, |uuid| {
                IngresoAbierto::Contratista(IngresoId::desde_uuid(uuid))
            })?);
        }
        for fila in self.filas_adentro(ADENTRO_PROVEEDORES).await? {
            adentro.extend(fila.a_persona(TABLA_INGRESO_PROVEEDOR, |uuid| {
                IngresoAbierto::Proveedor(IngresoProveedorId::desde_uuid(uuid))
            })?);
        }
        for fila in self.filas_adentro(ADENTRO_CORREO).await? {
            adentro.extend(fila.a_persona(TABLA_INGRESO_CORREO, |uuid| {
                IngresoAbierto::Correo(IngresoCorreoId::desde_uuid(uuid))
            })?);
        }
        adentro.sort_by(|a, b| {
            b.desde
                .cmp(&a.desde)
                .then_with(|| a.cedula.as_str().cmp(b.cedula.as_str()))
        });
        Ok(adentro)
    }

    async fn buscar_contratistas(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> Result<Vec<Contratista>, ErrorPersistencia> {
        self.buscar(
            "contratista",
            "cedula",
            criterio,
            limite,
            |fila: ContratistaLeido| Contratista::try_from(fila),
        )
        .await
    }

    async fn buscar_personal_kof(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> Result<Vec<PersonalKof>, ErrorPersistencia> {
        self.buscar(
            "personal_kof",
            "codigo_empleado",
            criterio,
            limite,
            |fila: PersonalKofLeido| PersonalKof::try_from(fila),
        )
        .await
    }

    async fn buscar_empresas(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> Result<Vec<Empresa>, ErrorPersistencia> {
        self.buscar("empresa", "id", criterio, limite, |fila: EmpresaLeida| {
            Empresa::try_from(fila)
        })
        .await
    }

    async fn buscar_empresas_proveedoras(
        &self,
        criterio: &Criterio,
        limite: usize,
    ) -> Result<Vec<EmpresaProveedora>, ErrorPersistencia> {
        self.buscar(
            "empresa_proveedora",
            "id",
            criterio,
            limite,
            |fila: EmpresaProveedoraLeida| EmpresaProveedora::try_from(fila),
        )
        .await
    }

    async fn historial_de(
        &self,
        registro: RegistroAuditado,
    ) -> Result<Vec<EntradaHistorial>, ErrorPersistencia> {
        self.auditoria_de(registro)
            .await?
            .into_iter()
            .map(|fila| {
                let accion = AccionAuditada::desde_codigo(&fila.accion).ok_or_else(|| {
                    dato_corrupto(
                        TABLA_AUDITORIA,
                        format!("acción desconocida: {}", fila.accion),
                    )
                })?;
                Ok(EntradaHistorial {
                    accion,
                    cambios: fila
                        .cambios
                        .into_iter()
                        .map(|cambio| CambioHistorial {
                            campo: cambio.campo,
                            antes: cambio.antes,
                            despues: cambio.despues,
                        })
                        .collect(),
                    operador: OperadorId::desde_uuid(fila.operador),
                    en: fila.en,
                })
            })
            .collect()
    }

    async fn listar_gafetes(
        &self,
        tipo: TipoGafete,
    ) -> Result<Vec<ResumenGafete>, ErrorPersistencia> {
        let mut respuesta = self
            .db()
            .query(
                "SELECT * OMIT id FROM gafete WHERE tipo = $tipo ORDER BY numero; \
                 SELECT VALUE id FROM prestamo_gafete;",
            )
            .bind(("tipo", tipo.codigo().to_owned()))
            .await
            .map_err(tecnica)?;
        let gafetes: Vec<GafeteDatos> = respuesta.take(0).map_err(tecnica)?;
        let ids_prestados: Vec<RecordId> = respuesta.take(1).map_err(tecnica)?;
        // La clave de cada préstamo es `TIPO-NÚMERO`.
        let en_prestamo: HashSet<String> = ids_prestados
            .into_iter()
            .filter_map(|id| match id.key {
                RecordIdKey::String(clave) => Some(clave),
                _ => None,
            })
            .collect();
        gafetes
            .into_iter()
            .map(|datos| {
                let gafete = Gafete::try_from(datos)?;
                let clave = clave_gafete(gafete.tipo(), gafete.numero());
                Ok(ResumenGafete {
                    prestado: en_prestamo.contains(&clave),
                    gafete,
                })
            })
            .collect()
    }

    async fn contratistas_con_praind_hasta(
        &self,
        hasta: NaiveDate,
        limite: usize,
    ) -> Result<Vec<Contratista>, ErrorPersistencia> {
        let limite = i64::try_from(limite).unwrap_or(i64::MAX);
        let mut respuesta = self
            .db()
            .query(
                "SELECT * FROM contratista \
                 WHERE tiene_acceso = true AND fecha_vencimiento_praind <= $hasta \
                 ORDER BY fecha_vencimiento_praind, nombre, cedula LIMIT $limite",
            )
            .bind(("hasta", hasta))
            .bind(("limite", limite))
            .await
            .map_err(tecnica)?;
        let filas: Vec<ContratistaLeido> = respuesta.take(0).map_err(tecnica)?;
        filas.into_iter().map(Contratista::try_from).collect()
    }
}
