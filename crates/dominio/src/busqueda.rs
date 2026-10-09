//! El buscador: qué escribe el operador, qué se considera una coincidencia
//! y en qué orden se muestran los resultados.
//!
//! Toda la regla vive acá y no en la interfaz ni en la base de datos, para
//! que cada adaptador y cada pantalla busquen igual. Las bases de datos sólo
//! sirven para **traer candidatos rápido**; quien decide si algo coincide y
//! en qué orden es [`relevantes`].
//!
//! # Qué se escribe
//!
//! - **Sólo números** (con o sin guiones, puntos o espacios): se busca por
//!   cédula o código de empleado. Una cédula de 10 dígitos que empieza en 0
//!   (formato del TSE) pierde ese cero, como en la regla A1.
//! - **Cualquier otra cosa**: se busca por nombre. Cada palabra escrita
//!   debe aparecer, en cualquier orden ("Carlos Sanches" encuentra a
//!   "Carlos Mauricio Sanches").
//! - Un texto vacío no busca nada.
//!
//! Se compara sin importar mayúsculas ni tildes, y la Ñ cuenta como N
//! ("pena" encuentra a "PEÑA"): quien opera teclea rápido y no debería
//! fallar por una letra.
//!
//! # Orden de los resultados
//!
//! Del mejor al peor [`Nivel`]; a igual nivel, por nombre y luego por
//! cédula (o código). Los resultados aproximados (con errores de tecleo)
//! sólo aparecen si no hay suficientes mejores.

use std::cmp::Ordering;

use crate::contratista::Contratista;
use crate::empresa::Empresa;
use crate::empresa_proveedora::EmpresaProveedora;
use crate::personal_kof::PersonalKof;

/// Qué tan buena es una coincidencia. Menor es mejor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nivel {
    /// La cédula o el código es exactamente lo escrito.
    Exacta,
    /// La cédula o el código empieza con lo escrito.
    PrefijoDeIdentificacion,
    /// Cada palabra escrita es una palabra completa del nombre.
    Palabra,
    /// Cada palabra escrita es el comienzo de alguna palabra del nombre.
    PrefijoDePalabra,
    /// Cada palabra escrita está dentro del nombre ("ndez" en "HERNANDEZ").
    Subcadena,
    /// Lo escrito está dentro de la cédula o del código.
    SubcadenaDeIdentificacion,
    /// Coincide si se perdonan errores de tecleo ("sanchez" por "SANCHES").
    Aproximada,
}

/// Con menos dígitos que esto, buscar "dentro" de la cédula trae demasiado
/// ruido: sólo se busca por el inicio.
pub const MINIMO_DIGITOS_PARA_SUBCADENA: usize = 4;

/// Cuántos errores de tecleo se perdonan en una palabra de `largo`
/// caracteres. Las palabras cortas no se perdonan: cualquier cosa de 3
/// letras se parece a demasiadas.
pub const fn tolerancia(largo: usize) -> usize {
    match largo {
        0..=3 => 0,
        4..=6 => 1,
        _ => 2,
    }
}

/// Mayúsculas, sin tildes ni diéresis, y la Ñ y la Ç como N y C. Es la forma
/// en que se compara y en que se guarda el texto para buscar.
pub fn plegar(texto: &str) -> String {
    texto
        .to_uppercase()
        .chars()
        .map(|letra| match letra {
            'Á' | 'À' | 'Ä' | 'Â' => 'A',
            'É' | 'È' | 'Ë' | 'Ê' => 'E',
            'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
            'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
            'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
            'Ñ' => 'N',
            'Ç' => 'C',
            otra => otra,
        })
        .collect()
}

/// Lo que se busca, ya normalizado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Criterio {
    /// Nada que buscar.
    Vacio,
    /// Cédula o código de empleado (sólo dígitos).
    Identificacion(String),
    /// Palabras que deben estar todas en el nombre (plegadas).
    Nombre(Vec<String>),
}

impl Criterio {
    /// Para personas: sólo números busca por cédula o código; lo demás, por
    /// nombre.
    pub fn desde_texto(texto: &str) -> Self {
        let sin_separadores: String = texto
            .chars()
            .filter(|c| !matches!(c, '-' | '.' | ' ' | '\t'))
            .collect();
        if sin_separadores.is_empty() {
            return Self::Vacio;
        }
        if sin_separadores.chars().all(|c| c.is_ascii_digit()) {
            let digitos = match sin_separadores.strip_prefix('0') {
                Some(resto) if sin_separadores.len() == 10 => resto.to_owned(),
                _ => sin_separadores,
            };
            return Self::Identificacion(digitos);
        }
        Self::de_nombre(texto)
    }

    /// Para cosas que sólo tienen nombre (empresas): aunque se escriban
    /// números ("3M"), se busca por nombre.
    pub fn de_nombre(texto: &str) -> Self {
        // Una palabra sin letras ni números (un "-" suelto) no busca nada.
        let palabras: Vec<String> = plegar(texto)
            .split_whitespace()
            .filter(|palabra| palabra.chars().any(char::is_alphanumeric))
            .map(str::to_owned)
            .collect();
        if palabras.is_empty() {
            Self::Vacio
        } else {
            Self::Nombre(palabras)
        }
    }

    pub const fn es_vacio(&self) -> bool {
        matches!(self, Self::Vacio)
    }

    /// El nivel de una coincidencia, o `None` si no coincide. `identificacion`
    /// es la cédula o el código; `nombre`, el nombre tal como está guardado.
    ///
    /// Es la definición de referencia: las bases de datos usan consultas
    /// para traer candidatos, y cada candidato pasa por acá.
    pub fn nivel(&self, identificacion: &str, nombre: &str) -> Option<Nivel> {
        match self {
            Self::Vacio => None,
            Self::Identificacion(digitos) => nivel_de_identificacion(digitos, identificacion),
            Self::Nombre(palabras) => nivel_de_nombre(palabras, &plegar(nombre)),
        }
    }
}

fn nivel_de_identificacion(digitos: &str, identificacion: &str) -> Option<Nivel> {
    if identificacion == digitos {
        Some(Nivel::Exacta)
    } else if identificacion.starts_with(digitos) {
        Some(Nivel::PrefijoDeIdentificacion)
    } else if digitos.len() >= MINIMO_DIGITOS_PARA_SUBCADENA && identificacion.contains(digitos) {
        Some(Nivel::SubcadenaDeIdentificacion)
    } else {
        None
    }
}

fn nivel_de_nombre(palabras: &[String], nombre_plegado: &str) -> Option<Nivel> {
    if palabras.is_empty() {
        return None;
    }
    let del_nombre: Vec<&str> = nombre_plegado.split_whitespace().collect();
    let todas = |cumple: &dyn Fn(&str) -> bool| palabras.iter().all(|palabra| cumple(palabra));

    if todas(&|w| del_nombre.contains(&w)) {
        Some(Nivel::Palabra)
    } else if todas(&|w| del_nombre.iter().any(|p| p.starts_with(w))) {
        Some(Nivel::PrefijoDePalabra)
    } else if todas(&|w| nombre_plegado.contains(w)) {
        Some(Nivel::Subcadena)
    } else if todas(&|w| nombre_plegado.contains(w) || del_nombre.iter().any(|p| casi_igual(w, p)))
    {
        Some(Nivel::Aproximada)
    } else {
        None
    }
}

/// Si lo escrito se parece a una palabra del nombre, perdonando errores de
/// tecleo: a la palabra completa o a su comienzo (por si todavía se está
/// escribiendo).
pub fn casi_igual(escrita: &str, del_nombre: &str) -> bool {
    let largo = escrita.chars().count();
    let permitidos = tolerancia(largo);
    if permitidos == 0 {
        return false;
    }
    let comienzo: String = del_nombre.chars().take(largo).collect();
    distancia(escrita, del_nombre) <= permitidos || distancia(escrita, &comienzo) <= permitidos
}

/// Distancia de edición con transposición de letras vecinas (OSA): cuántos
/// cambios, borrados, inserciones o intercambios separan a dos textos.
pub fn distancia(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // Se guardan sólo las dos filas anteriores de la tabla de distancias.
    let en = |fila: &[usize], columna: usize| fila.get(columna).copied().unwrap_or(0);
    let mut anterior_de_la_anterior: Vec<usize> = Vec::new();
    let mut anterior: Vec<usize> = (0..=b.len()).collect();
    for (i, letra_a) in a.iter().enumerate() {
        let mut actual = vec![i + 1];
        for (j, letra_b) in b.iter().enumerate() {
            let costo = usize::from(letra_a != letra_b);
            let mut mejor = (en(&anterior, j + 1) + 1)
                .min(en(&actual, j) + 1)
                .min(en(&anterior, j) + costo);
            let intercambio =
                i > 0 && j > 0 && b.get(j - 1) == Some(letra_a) && a.get(i - 1) == Some(letra_b);
            if intercambio {
                mejor = mejor.min(en(&anterior_de_la_anterior, j - 1) + 1);
            }
            actual.push(mejor);
        }
        anterior_de_la_anterior = anterior;
        anterior = actual;
    }
    en(&anterior, b.len())
}

/// Lo que se necesita saber de un candidato para decidir si coincide: su
/// cédula o código y su nombre.
pub trait Buscable {
    fn identificacion(&self) -> String;
    fn nombre(&self) -> String;
}

impl Buscable for Contratista {
    fn identificacion(&self) -> String {
        self.cedula().to_string()
    }

    fn nombre(&self) -> String {
        self.nombre().to_string()
    }
}

impl Buscable for PersonalKof {
    fn identificacion(&self) -> String {
        self.codigo().to_string()
    }

    fn nombre(&self) -> String {
        self.nombre().to_string()
    }
}

/// Las empresas sólo se buscan por nombre; su ID sirve de desempate.
impl Buscable for Empresa {
    fn identificacion(&self) -> String {
        self.id().to_string()
    }

    fn nombre(&self) -> String {
        self.nombre().to_string()
    }
}

impl Buscable for EmpresaProveedora {
    fn identificacion(&self) -> String {
        self.id().to_string()
    }

    fn nombre(&self) -> String {
        self.nombre().to_string()
    }
}

/// De los `candidatos`, los que coinciden con el criterio, del mejor al
/// peor, hasta `limite`. Es lo que ven las pantallas.
///
/// Es la única función que decide el orden: los adaptadores sólo se
/// ocupan de traer candidatos (cuantos más mejor, nunca menos de los que
/// coinciden).
pub fn relevantes<T: Buscable>(
    criterio: &Criterio,
    candidatos: impl IntoIterator<Item = T>,
    limite: usize,
) -> Vec<T> {
    let mut con_nivel: Vec<(Nivel, String, String, T)> = candidatos
        .into_iter()
        .filter_map(|candidato| {
            let identificacion = candidato.identificacion();
            let nombre = candidato.nombre();
            criterio
                .nivel(&identificacion, &nombre)
                .map(|nivel| (nivel, nombre, identificacion, candidato))
        })
        .collect();
    con_nivel.sort_by(|a, b| orden(a, b));
    con_nivel
        .into_iter()
        .take(limite)
        .map(|(_, _, _, candidato)| candidato)
        .collect()
}

fn orden<T>(a: &(Nivel, String, String, T), b: &(Nivel, String, String, T)) -> Ordering {
    a.0.cmp(&b.0)
        .then_with(|| a.1.cmp(&b.1))
        .then_with(|| a.2.cmp(&b.2))
}

/// Cuántos de los `candidatos` coinciden con un nivel mejor o igual a
/// `hasta`. Los adaptadores lo usan para saber si ya tienen suficientes y
/// pueden evitar las etapas más caras de la búsqueda.
pub fn cuantos_hasta<T: Buscable>(criterio: &Criterio, candidatos: &[T], hasta: Nivel) -> usize {
    candidatos
        .iter()
        .filter(|candidato| {
            criterio
                .nivel(&candidato.identificacion(), &candidato.nombre())
                .is_some_and(|nivel| nivel <= hasta)
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Persona(&'static str, &'static str);

    impl Buscable for Persona {
        fn identificacion(&self) -> String {
            self.0.to_owned()
        }

        fn nombre(&self) -> String {
            self.1.to_owned()
        }
    }

    fn buscar(texto: &str, personas: &[Persona], limite: usize) -> Vec<Persona> {
        relevantes(&Criterio::desde_texto(texto), personas.to_vec(), limite)
    }

    const NINGUNA: [&str; 0] = [];

    fn ids(encontradas: &[Persona]) -> Vec<&'static str> {
        encontradas.iter().map(|p| p.0).collect()
    }

    // --- Cómo se entiende lo escrito ---

    #[test]
    fn un_texto_vacio_o_de_separadores_no_busca() {
        for texto in ["", "   ", "-", " - . "] {
            assert_eq!(Criterio::desde_texto(texto), Criterio::Vacio, "{texto:?}");
            assert_eq!(Criterio::de_nombre(texto), Criterio::Vacio, "{texto:?}");
        }
        assert_eq!(Criterio::Vacio.nivel("1", "ANA"), None);
        assert!(Criterio::Vacio.es_vacio());
    }

    #[test]
    fn los_numeros_buscan_por_identificacion_y_el_resto_por_nombre() {
        assert_eq!(
            Criterio::desde_texto(" 1.1111 1111 "),
            Criterio::Identificacion("111111111".into())
        );
        assert_eq!(
            Criterio::desde_texto("josé  pe"),
            Criterio::Nombre(vec!["JOSE".into(), "PE".into()])
        );
        // Mezclar letras y números no es una identificación.
        assert_eq!(
            Criterio::desde_texto("ana 2"),
            Criterio::Nombre(vec!["ANA".into(), "2".into()])
        );
        // Para empresas, los números también son parte del nombre.
        assert_eq!(
            Criterio::de_nombre("3M"),
            Criterio::Nombre(vec!["3M".into()])
        );
    }

    #[test]
    fn la_cedula_del_tse_pierde_el_cero_como_en_la_regla_a1() {
        assert_eq!(
            Criterio::desde_texto("0111111111"),
            Criterio::Identificacion("111111111".into())
        );
        assert_eq!(
            Criterio::desde_texto("0111"),
            Criterio::Identificacion("0111".into()),
            "con menos de 10 dígitos el cero cuenta"
        );
    }

    #[test]
    fn plegar_quita_tildes_y_cuenta_la_enie_como_n() {
        assert_eq!(plegar("josé peña ÇAÜ"), "JOSE PENA CAU");
    }

    // --- Niveles ---

    #[test]
    fn los_niveles_de_identificacion() {
        let criterio = Criterio::desde_texto("1111");
        assert_eq!(criterio.nivel("1111", "X"), Some(Nivel::Exacta));
        assert_eq!(
            criterio.nivel("111122223", "X"),
            Some(Nivel::PrefijoDeIdentificacion)
        );
        assert_eq!(
            criterio.nivel("911112223", "X"),
            Some(Nivel::SubcadenaDeIdentificacion),
            "dentro de la cédula, con 4 dígitos o más"
        );
        assert_eq!(criterio.nivel("222233334", "X"), None);
        assert_eq!(
            Criterio::desde_texto("111").nivel("911112223", "X"),
            None,
            "con 3 dígitos sólo se busca por el inicio"
        );
    }

    #[test]
    fn los_niveles_de_nombre() {
        let nivel = |texto: &str, nombre: &str| Criterio::desde_texto(texto).nivel("1", nombre);
        assert_eq!(nivel("jose", "JOSE PEÑA"), Some(Nivel::Palabra));
        assert_eq!(
            nivel("peña jose", "JOSE PEÑA"),
            Some(Nivel::Palabra),
            "cualquier orden"
        );
        assert_eq!(nivel("jos pe", "JOSE PEÑA"), Some(Nivel::PrefijoDePalabra));
        assert_eq!(nivel("ndez", "ANA HERNANDEZ"), Some(Nivel::Subcadena));
        assert_eq!(nivel("sanchez", "CARLOS SANCHES"), Some(Nivel::Aproximada));
        assert_eq!(nivel("zzz", "JOSE PEÑA"), None);
    }

    #[test]
    fn las_palabras_de_una_vez_pueden_estar_separadas_en_el_nombre() {
        assert_eq!(
            Criterio::desde_texto("carlos sanches").nivel("1", "CARLOS MAURICIO SANCHES"),
            Some(Nivel::Palabra),
            "el nombre del medio no estorba"
        );
    }

    #[test]
    fn la_enie_se_encuentra_con_n_y_con_enie() {
        for texto in ["pena", "peña", "PEÑA", "PENA"] {
            assert_eq!(
                Criterio::desde_texto(texto).nivel("1", "JOSE PEÑA"),
                Some(Nivel::Palabra),
                "{texto}"
            );
        }
    }

    #[test]
    fn los_errores_de_tecleo_se_perdonan_segun_el_largo_de_la_palabra() {
        let nivel = |texto: &str, nombre: &str| Criterio::desde_texto(texto).nivel("1", nombre);
        // 4 a 6 letras: un error.
        assert_eq!(
            nivel("jsoe", "JOSE"),
            Some(Nivel::Aproximada),
            "letras cambiadas"
        );
        assert_eq!(
            nivel("joze", "JOSE"),
            Some(Nivel::Aproximada),
            "una letra distinta"
        );
        assert_eq!(nivel("jse", "JOSE"), None, "3 letras no se perdonan");
        assert_eq!(nivel("mraia", "MARIA"), Some(Nivel::Aproximada));
        // Dos errores en una palabra corta no.
        assert_eq!(nivel("jzze", "JOSE"), None);
        // 7 letras o más: dos errores.
        assert_eq!(nivel("hernadnes", "HERNANDEZ"), Some(Nivel::Aproximada));
        assert_eq!(nivel("hrnandz", "HERNANDEZ"), Some(Nivel::Aproximada));
        // Mientras se escribe: contra el comienzo de la palabra.
        assert_eq!(nivel("hernamd", "HERNANDEZ"), Some(Nivel::Aproximada));
    }

    #[test]
    fn distancia_cuenta_cambios_borrados_inserciones_e_intercambios() {
        assert_eq!(distancia("", ""), 0);
        assert_eq!(distancia("abc", "abc"), 0);
        assert_eq!(distancia("abc", ""), 3);
        assert_eq!(distancia("", "abc"), 3);
        assert_eq!(distancia("jose", "joze"), 1, "cambio");
        assert_eq!(distancia("jose", "jos"), 1, "borrado");
        assert_eq!(distancia("jose", "josee"), 1, "inserción");
        assert_eq!(distancia("jsoe", "jose"), 1, "intercambio de vecinas");
        assert_eq!(distancia("ca", "abc"), 3, "OSA, no la distancia completa");
        assert_eq!(distancia("sanchez", "sanches"), 1);
    }

    // --- Orden ---

    #[test]
    fn los_mejores_niveles_van_primero_y_luego_por_nombre_y_cedula() {
        let personas = [
            Persona("1", "ANA HERNANDEZ"),
            Persona("2", "HERNAN MORA"),
            Persona("3", "HERNAN ALVAREZ"),
            Persona("4", "CARLOS HERNAN"),
            Persona("5", "SOFIA ZHERNAN"),
        ];
        // "hernan" es palabra completa en 2, 3 y 4; comienzo de palabra en 1;
        // y sólo está dentro de la palabra en 5.
        assert_eq!(
            ids(&buscar("hernan", &personas, 10)),
            ["4", "3", "2", "1", "5"],
            "palabras (CARLOS, HERNAN ALVAREZ, HERNAN MORA), luego comienzo, luego dentro"
        );
        assert_eq!(
            ids(&buscar("hernan", &personas, 2)),
            ["4", "3"],
            "el límite corta al final"
        );
    }

    #[test]
    fn la_cedula_exacta_va_antes_que_las_que_solo_empiezan_igual() {
        let personas = [
            Persona("111111111", "ZOE"),
            Persona("1111", "BETO"),
            Persona("111122223", "ANA"),
            Persona("911112223", "LUIS"),
        ];
        assert_eq!(
            ids(&buscar("1111", &personas, 10)),
            ["1111", "111122223", "111111111", "911112223"]
        );
    }

    #[test]
    fn el_empate_se_resuelve_por_cedula() {
        let personas = [Persona("2", "ANA"), Persona("1", "ANA")];
        assert_eq!(ids(&buscar("ana", &personas, 10)), ["1", "2"]);
    }

    #[test]
    fn lo_aproximado_va_al_final_y_solo_si_hay_lugar() {
        let personas = [
            Persona("1", "SANCHEZ ANA"),
            Persona("2", "ROSA SANCHES"),
            Persona("3", "MARIO SANCHEZ"),
        ];
        // "sanchez" es palabra en 1 y 3; en 2 sólo es parecida.
        assert_eq!(ids(&buscar("sanchez", &personas, 10)), ["3", "1", "2"]);
        assert_eq!(ids(&buscar("sanchez", &personas, 2)), ["3", "1"]);
    }

    #[test]
    fn cuantos_hasta_cuenta_los_que_alcanzan_el_nivel() {
        let personas = [
            Persona("1", "ANA HERNANDEZ"),
            Persona("2", "HERNAN MORA"),
            Persona("3", "SOFIA ZHERNAN"),
        ];
        let criterio = Criterio::desde_texto("hernan");
        assert_eq!(cuantos_hasta(&criterio, &personas, Nivel::Palabra), 1);
        assert_eq!(
            cuantos_hasta(&criterio, &personas, Nivel::PrefijoDePalabra),
            2
        );
        assert_eq!(cuantos_hasta(&criterio, &personas, Nivel::Aproximada), 3);
    }

    #[test]
    fn sin_criterio_o_sin_candidatos_no_hay_resultados() {
        let ana = [Persona("1", "ANA")];
        assert_eq!(ids(&buscar("", &ana, 10)), NINGUNA, "texto vacío");
        assert_eq!(ids(&buscar("ana", &[], 10)), NINGUNA, "sin candidatos");
        assert_eq!(ids(&buscar("ana", &ana, 0)), NINGUNA, "sin cupo");
    }
}
