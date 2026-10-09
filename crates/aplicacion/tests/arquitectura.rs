//! Vigila la regla de dependencias: la aplicación sólo puede usar el
//! dominio y crates puras. Si alguien le agrega `SurrealDB`, Tauri o la red,
//! esta prueba falla y lo dice.

#[cfg(test)]
mod tests {
    const PERMITIDAS: [&str; 4] = ["limen-dominio", "chrono", "thiserror", "uuid"];

    #[test]
    fn la_aplicacion_solo_depende_del_dominio_y_crates_puras() {
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
                "la aplicación no puede depender de `{nombre}`: sólo de {PERMITIDAS:?}"
            );
        }
    }
}
