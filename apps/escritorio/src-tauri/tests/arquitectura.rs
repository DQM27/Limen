//! Vigila la regla de dependencias: el cascarón de Tauri sólo conecta la
//! interfaz con `limen-escritorio-comandos` y arma la aplicación con
//! `composicion`. No habla con el dominio, la aplicación ni la base de datos
//! por su cuenta: si alguien agrega `limen-dominio` o `surrealdb`, esta
//! prueba falla.
//!
//! Además, el cascarón no lleva pruebas propias a propósito: un ejecutable de
//! pruebas que enlaza Tauri no arranca en Windows (falta el manifiesto de
//! comctl32), así que toda la lógica vive en `comandos`, que sí se prueba.

#[cfg(test)]
mod tests {
    const PERMITIDAS: [&str; 6] = [
        "limen-composicion",
        "limen-escritorio-comandos",
        "limen-infra-plataforma",
        "limen-infra-surreal",
        "tauri",
        "tauri-plugin-log",
    ];

    #[test]
    fn el_cascaron_no_depende_del_dominio_ni_de_la_base_directamente() {
        let manifiesto = include_str!("../Cargo.toml");
        let dependencias = manifiesto
            .split("\n[dependencies]")
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
                "el cascarón no puede depender de `{nombre}`: sólo de {PERMITIDAS:?}"
            );
        }
    }
}
