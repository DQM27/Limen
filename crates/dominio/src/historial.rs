//! El historial de ingresos: cómo se ordena lo que juntan las cuatro vías y cuánto se trae.
//!
//! Cada vía entrega lo suyo ya del más reciente al más antiguo; al juntarlas hay que volver a
//! ordenar. Lo decide el dominio para que el escritorio y el panel muestren lo mismo.

use std::cmp::Reverse;

use chrono::{DateTime, Utc};

/// Cuántos movimientos trae el historial como máximo: más no se leen en una pantalla, y quien
/// necesite todo acota las fechas.
pub const MAXIMO_MOVIMIENTOS: usize = 20_000;

/// El orden en que se muestran los movimientos: del más reciente al más antiguo y, a igual
/// instante, por cédula o código. Recibe la entrada y la identidad de cada uno y devuelve sus
/// posiciones, ya en orden.
pub fn orden_del_historial(movimientos: &[(DateTime<Utc>, String)]) -> Vec<usize> {
    let mut orden: Vec<usize> = (0..movimientos.len()).collect();
    orden.sort_by(|&a, &b| {
        let clave = |i: usize| {
            movimientos
                .get(i)
                .map(|(en, identidad)| (Reverse(*en), identidad))
        };
        clave(a).cmp(&clave(b))
    });
    orden
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    #[test]
    fn lo_mas_reciente_va_primero_y_el_empate_se_resuelve_por_identidad() {
        let movimientos = vec![
            (instante("2026-10-14T10:00:00Z"), "222".to_owned()),
            (instante("2026-10-14T12:00:00Z"), "333".to_owned()),
            (instante("2026-10-14T10:00:00Z"), "111".to_owned()),
        ];
        assert_eq!(orden_del_historial(&movimientos), [1, 2, 0]);
        assert_eq!(orden_del_historial(&[]), Vec::<usize>::new());
    }
}
