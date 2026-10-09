//! PRAIND: los cursos de seguridad, con fecha de vencimiento. Todo
//! contratista lo necesita al día, sea PRAIND o IN HOUSE.

use chrono::NaiveDate;

/// Días antes del vencimiento en que la entrada se permite con advertencia.
pub const DIAS_ADVERTENCIA_PRAIND: i64 = 30;

/// Un PRAIND cuya fecha ya pasó (`< hoy`) no habilita a nadie. Vencer HOY
/// todavía cuenta como vigente.
pub fn praind_vencido(fecha_vencimiento: NaiveDate, hoy: NaiveDate) -> bool {
    fecha_vencimiento < hoy
}

/// Días que faltan para el vencimiento (negativo si ya venció).
pub fn dias_para_vencer(fecha_vencimiento: NaiveDate, hoy: NaiveDate) -> i64 {
    (fecha_vencimiento - hoy).num_days()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    #[test]
    fn vencido_solo_si_la_fecha_ya_paso() {
        let hoy = fecha("2026-09-27");
        assert!(praind_vencido(fecha("2026-09-26"), hoy));
        assert!(!praind_vencido(hoy, hoy), "vencer hoy todavía es vigente");
        assert!(!praind_vencido(fecha("2027-01-01"), hoy));
    }

    #[test]
    fn cuenta_los_dias_que_faltan() {
        let hoy = fecha("2026-09-27");
        assert_eq!(dias_para_vencer(fecha("2026-10-27"), hoy), 30);
        assert_eq!(dias_para_vencer(hoy, hoy), 0);
        assert_eq!(dias_para_vencer(fecha("2026-09-26"), hoy), -1);
    }
}
