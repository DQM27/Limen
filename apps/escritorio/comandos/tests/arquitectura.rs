//! Vigila la regla de dependencias: este adaptador de entrada habla con la
//! `Aplicacion` y el dominio, nunca con la base de datos ni con Tauri. Si
//! alguien agrega `SurrealDB`, un adaptador `infra-*` o el propio Tauri, esta
//! prueba falla y lo dice.

#[cfg(test)]
mod tests {
    const PERMITIDAS: [&str; 9] = [
        "limen-aplicacion",
        "limen-composicion",
        "limen-dominio",
        "chrono",
        "log",
        "serde",
        "thiserror",
        "uuid",
        // Sólo en pruebas: ver [dev-dependencies].
        "tokio",
    ];

    #[test]
    fn los_comandos_no_dependen_de_la_base_ni_de_tauri() {
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
                "los comandos no pueden depender de `{nombre}`: sólo de {PERMITIDAS:?}"
            );
        }
    }
}
