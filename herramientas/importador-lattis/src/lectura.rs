//! Lee un volcado de la forma
//! `INSERT INTO tabla (a, b) VALUES ('x', NULL), ('y', 'z');`
//! que es lo que genera el volcado de Lattis (`format('%L')` de Postgres):
//! valores entre comillas simples con `''` como escape, o `NULL`. Un valor
//! puede traer saltos de línea, así que se lee carácter por carácter.

use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorLectura {
    #[error("no encontré la palabra VALUES")]
    SinValores,
    #[error("la cabecera del INSERT no tiene columnas")]
    SinColumnas,
    #[error("formato inesperado en la fila {0}")]
    Formato(usize),
    #[error("la fila {fila} tiene {tiene} valores y se esperaban {esperados}")]
    Anchura {
        fila: usize,
        tiene: usize,
        esperados: usize,
    },
    #[error("falta la columna `{0}`")]
    ColumnaFaltante(String),
    #[error("la columna `{0}` no puede estar vacía")]
    ValorNulo(String),
}

/// Las filas de un `INSERT`, con los nombres de sus columnas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tabla {
    columnas: Vec<String>,
    filas: Vec<Vec<Option<String>>>,
}

impl Tabla {
    pub fn leer(sql: &str) -> Result<Self, ErrorLectura> {
        let (cabecera, cuerpo) = sql.split_once("VALUES").ok_or(ErrorLectura::SinValores)?;
        let columnas = columnas_de(cabecera)?;
        let mut caracteres = cuerpo.chars().peekable();
        let mut filas = Vec::new();
        loop {
            saltar_blancos(&mut caracteres);
            match caracteres.next() {
                Some('(') => {}
                Some(',') => continue,
                Some(';') | None => break,
                Some(_) => return Err(ErrorLectura::Formato(filas.len() + 1)),
            }
            let valores = leer_fila(&mut caracteres, filas.len() + 1)?;
            if valores.len() != columnas.len() {
                return Err(ErrorLectura::Anchura {
                    fila: filas.len() + 1,
                    tiene: valores.len(),
                    esperados: columnas.len(),
                });
            }
            filas.push(valores);
        }
        Ok(Self { columnas, filas })
    }

    pub const fn len(&self) -> usize {
        self.filas.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.filas.is_empty()
    }

    pub fn filas(&self) -> impl Iterator<Item = Fila<'_>> {
        self.filas.iter().map(|valores| Fila {
            columnas: &self.columnas,
            valores,
        })
    }
}

/// Una fila: se piden los valores por el nombre de su columna.
#[derive(Debug, Clone, Copy)]
pub struct Fila<'a> {
    columnas: &'a [String],
    valores: &'a [Option<String>],
}

impl<'a> Fila<'a> {
    /// El valor de la columna, o `None` si es `NULL`.
    pub fn opcional(&self, columna: &str) -> Result<Option<&'a str>, ErrorLectura> {
        let posicion = self
            .columnas
            .iter()
            .position(|nombre| nombre == columna)
            .ok_or_else(|| ErrorLectura::ColumnaFaltante(columna.to_owned()))?;
        Ok(self
            .valores
            .get(posicion)
            .and_then(|valor| valor.as_deref()))
    }

    /// El valor de la columna; si es `NULL` es un error.
    pub fn texto(&self, columna: &str) -> Result<&'a str, ErrorLectura> {
        self.opcional(columna)?
            .ok_or_else(|| ErrorLectura::ValorNulo(columna.to_owned()))
    }
}

fn columnas_de(cabecera: &str) -> Result<Vec<String>, ErrorLectura> {
    let (_, resto) = cabecera.split_once('(').ok_or(ErrorLectura::SinColumnas)?;
    let (lista, _) = resto.split_once(')').ok_or(ErrorLectura::SinColumnas)?;
    let columnas: Vec<String> = lista
        .split(',')
        .map(|columna| columna.trim().to_owned())
        .filter(|columna| !columna.is_empty())
        .collect();
    if columnas.is_empty() {
        return Err(ErrorLectura::SinColumnas);
    }
    Ok(columnas)
}

fn saltar_blancos(caracteres: &mut Peekable<Chars<'_>>) {
    while caracteres.next_if(|c| c.is_whitespace()).is_some() {}
}

/// Lee los valores de una fila, ya consumido el `(`, hasta el `)`.
fn leer_fila(
    caracteres: &mut Peekable<Chars<'_>>,
    fila: usize,
) -> Result<Vec<Option<String>>, ErrorLectura> {
    let mut valores = Vec::new();
    loop {
        saltar_blancos(caracteres);
        valores.push(leer_valor(caracteres, fila)?);
        saltar_blancos(caracteres);
        match caracteres.next() {
            Some(',') => {}
            Some(')') => return Ok(valores),
            _ => return Err(ErrorLectura::Formato(fila)),
        }
    }
}

/// Un valor: texto entre comillas simples (`Some`) o `NULL` (`None`). Cualquier
/// otra cosa, o un texto que no cierra, es un error de formato.
fn leer_valor(
    caracteres: &mut Peekable<Chars<'_>>,
    fila: usize,
) -> Result<Option<String>, ErrorLectura> {
    let error = ErrorLectura::Formato(fila);
    if caracteres.next_if_eq(&'\'').is_some() {
        let mut texto = String::new();
        loop {
            match caracteres.next().ok_or_else(|| error.clone())? {
                '\'' if caracteres.next_if_eq(&'\'').is_some() => texto.push('\''),
                '\'' => return Ok(Some(texto)),
                otro => texto.push(otro),
            }
        }
    }
    let mut palabra = String::new();
    while let Some(letra) = caracteres.next_if(char::is_ascii_alphabetic) {
        palabra.push(letra);
    }
    if palabra == "NULL" {
        Ok(None)
    } else {
        Err(error)
    }
}
