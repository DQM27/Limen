//! Consultas de lectura sobre `SurrealDB` (puerto `Consultas`).

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use limen_aplicacion::puertos::{Consultas, ErrorPersistencia, IngresoAbierto, PersonaAdentro};
use limen_dominio::busqueda::{
    Buscable, Criterio, MINIMO_DIGITOS_PARA_SUBCADENA, Nivel, cuantos_hasta, relevantes, tolerancia,
};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::Empresa;
use limen_dominio::empresa_proveedora::EmpresaProveedora;
use limen_dominio::ingreso_contratista::IngresoId;
use limen_dominio::ingreso_correo::IngresoCorreoId;
use limen_dominio::ingreso_proveedor::IngresoProveedorId;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::personal_kof::PersonalKof;
use surrealdb::types::{RecordId, SurrealValue};

use crate::almacen::AlmacenSurreal;
use crate::error::{dato_corrupto, tecnica};
use crate::registros::{
    ContratistaLeido, EmpresaLeida, EmpresaProveedoraLeida, PersonalKofLeido,
    TABLA_INGRESO_CONTRATISTA, TABLA_INGRESO_CORREO, TABLA_INGRESO_PROVEEDOR, medio_de, numero_de,
    uuid_de,
};

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
    donde: String,
    enlaces: Vec<(String, String)>,
    /// Hasta qué nivel garantiza esta etapa traer **todos** los que
    /// coinciden. Si ya hay suficientes resultados de ese nivel o mejor, las
    /// etapas siguientes (más caras) no pueden cambiar el resultado.
    completa_hasta: Nivel,
}

/// Las etapas de una búsqueda, de la más barata a la más cara.
///
/// - **Nombre**: primero el índice de texto (cada palabra escrita es el
///   comienzo de una palabra del nombre); si faltan resultados, lo que está
///   dentro del nombre; si todavía faltan, lo parecido (errores de tecleo,
///   con las funciones de distancia de la propia base).
/// - **Cédula o código**: primero el rango del índice (empieza igual); si
///   faltan resultados, lo que la contiene.
fn etapas(criterio: &Criterio, campo_identificacion: &str) -> Vec<Etapa> {
    match criterio {
        Criterio::Vacio => Vec::new(),
        Criterio::Identificacion(digitos) => {
            let mut etapas = vec![Etapa {
                donde: format!("{campo_identificacion} >= $p0 AND {campo_identificacion} < $p1"),
                enlaces: vec![
                    ("p0".to_owned(), digitos.clone()),
                    ("p1".to_owned(), siguiente(digitos)),
                ],
                completa_hasta: Nivel::PrefijoDeIdentificacion,
            }];
            if digitos.len() >= MINIMO_DIGITOS_PARA_SUBCADENA {
                etapas.push(Etapa {
                    donde: format!("{campo_identificacion} CONTAINS $p0"),
                    enlaces: vec![("p0".to_owned(), digitos.clone())],
                    completa_hasta: Nivel::SubcadenaDeIdentificacion,
                });
            }
            etapas
        }
        Criterio::Nombre(palabras) if palabras.is_empty() => Vec::new(),
        Criterio::Nombre(palabras) => {
            let enlaces: Vec<(String, String)> = palabras
                .iter()
                .enumerate()
                .map(|(i, palabra)| (format!("p{i}"), palabra.clone()))
                .collect();
            let contiene = |i: usize| format!("nombre_busqueda CONTAINS $p{i}");
            let todas = |clausula: &dyn Fn(usize, &str) -> String| {
                palabras
                    .iter()
                    .enumerate()
                    .map(|(i, palabra)| clausula(i, palabra))
                    .collect::<Vec<_>>()
                    .join(" AND ")
            };
            vec![
                Etapa {
                    donde: "nombre_busqueda @AND@ $texto".to_owned(),
                    enlaces: vec![("texto".to_owned(), palabras.join(" "))],
                    completa_hasta: Nivel::PrefijoDePalabra,
                },
                Etapa {
                    donde: todas(&|i, _| contiene(i)),
                    enlaces: enlaces.clone(),
                    completa_hasta: Nivel::Subcadena,
                },
                Etapa {
                    donde: todas(&|i, palabra| parecida(i, palabra)),
                    enlaces,
                    completa_hasta: Nivel::Aproximada,
                },
            ]
        }
    }
}

/// La condición de que una palabra escrita esté dentro del nombre o se
/// parezca a alguna de sus palabras (o a su comienzo). Es la misma regla que
/// `limen_dominio::busqueda::casi_igual`, escrita con las funciones de la
/// base.
fn parecida(i: usize, palabra: &str) -> String {
    let largo = palabra.chars().count();
    let permitidos = tolerancia(largo);
    if permitidos == 0 {
        return format!("nombre_busqueda CONTAINS $p{i}");
    }
    format!(
        "(nombre_busqueda CONTAINS $p{i} OR array::any(string::split(nombre_busqueda, ' '), \
         |$q| string::distance::osa($q, $p{i}) <= {permitidos} \
         OR string::distance::osa(string::slice($q, 0, {largo}), $p{i}) <= {permitidos}))"
    )
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
            let mut consulta = self
                .db()
                .query(format!("SELECT * FROM {tabla} WHERE {}", etapa.donde));
            for enlace in etapa.enlaces {
                consulta = consulta.bind(enlace);
            }
            let mut respuesta = consulta.await.map_err(tecnica)?;
            let filas: Vec<L> = respuesta.take(0).map_err(tecnica)?;
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
}
