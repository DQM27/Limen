//! Vigila la regla de dependencias: el dominio sólo puede usar las crates
//! puras de esta lista. Si alguien le agrega una base de datos, la red o una
//! interfaz, esta prueba falla y lo dice.

const PERMITIDAS: [&str; 3] = ["chrono", "thiserror", "uuid"];

#[test]
fn el_dominio_solo_depende_de_crates_puras() {
    let manifiesto = include_str!("../Cargo.toml");
    let dependencias = manifiesto
        .split("[dependencies]")
        .nth(1)
        .expect("el manifiesto tiene [dependencies]")
        .split("\n[")
        .next()
        .unwrap_or_default();
    for linea in dependencias.lines() {
        let linea = linea.trim();
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let nombre = linea.split(['.', '=', ' ']).next().unwrap_or_default();
        assert!(
            PERMITIDAS.contains(&nombre),
            "el dominio no puede depender de `{nombre}`: sólo de {PERMITIDAS:?}"
        );
    }
}
