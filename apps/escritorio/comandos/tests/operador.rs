//! El operador provisional (TEMPORAL, ver `src/operador.rs`): se crea una
//! vez, queda guardado en el equipo y sin él no se registra nada.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use limen_aplicacion::errores::MENSAJE_ERROR_TECNICO;
    use limen_aplicacion::puertos::GeneradorIds;
    use limen_aplicacion::sesion::OperadorId;
    use limen_escritorio_comandos::{ErrorJson, ErrorOperador, OperadorDelEquipo, TipoErrorJson};
    use uuid::Uuid;

    /// Siempre el mismo identificador, para poder comprobarlo.
    struct IdFijo(Uuid);

    impl GeneradorIds for IdFijo {
        fn nuevo(&self) -> Uuid {
            self.0
        }
    }

    fn siete() -> IdFijo {
        IdFijo(Uuid::from_u128(7))
    }

    fn ruta_nueva() -> PathBuf {
        let carpeta = std::env::temp_dir().join(format!("limen-operador-{}", Uuid::now_v7()));
        fs::create_dir_all(&carpeta).unwrap();
        carpeta.join("operador.json")
    }

    #[test]
    fn sin_archivo_no_hay_operador_y_no_se_registra_nada() {
        let equipo = OperadorDelEquipo::abrir(ruta_nueva()).unwrap();
        assert_eq!(equipo.actual(), None);
        assert_eq!(equipo.sesion().unwrap_err(), ErrorJson::sin_operador());
    }

    #[test]
    fn el_operador_creado_queda_guardado_y_se_lee_al_reabrir() {
        let ruta = ruta_nueva();
        let equipo = OperadorDelEquipo::abrir(ruta.clone()).unwrap();
        let creado = equipo.crear(&siete(), "  Ana   María ").unwrap();
        assert_eq!(creado.nombre, "Ana María", "sin espacios de más");
        assert_eq!(equipo.actual(), Some(creado.clone()), "ya hay sesión");

        let reabierto = OperadorDelEquipo::abrir(ruta).unwrap();
        assert_eq!(reabierto.actual(), Some(creado), "sobrevive al reinicio");
    }

    #[test]
    fn la_sesion_lleva_el_identificador_del_operador() {
        let equipo = OperadorDelEquipo::abrir(ruta_nueva()).unwrap();
        equipo.crear(&siete(), "Ana").unwrap();
        assert_eq!(
            equipo.sesion().unwrap().operador(),
            OperadorId::desde_uuid(Uuid::from_u128(7))
        );
    }

    #[test]
    fn el_nombre_vacio_se_rechaza_y_no_guarda_nada() {
        let ruta = ruta_nueva();
        let equipo = OperadorDelEquipo::abrir(ruta.clone()).unwrap();
        let error = equipo.crear(&siete(), "   ").unwrap_err();
        assert_eq!(error, ErrorOperador::NombreVacio);
        assert_eq!(equipo.actual(), None);
        assert!(!ruta.exists(), "no se escribió el archivo");

        let json = ErrorJson::from(error);
        assert_eq!(json.tipo, TipoErrorJson::Negocio);
        assert_eq!(json.codigo, "nombre_vacio");
    }

    #[test]
    fn un_archivo_danado_es_una_falla_tecnica_sin_detalle_para_la_pantalla() {
        let ruta = ruta_nueva();
        fs::write(&ruta, "esto no es json").unwrap();
        let error = OperadorDelEquipo::abrir(ruta).unwrap_err();
        assert!(matches!(error, ErrorOperador::Archivo(_)), "{error:?}");

        let json = ErrorJson::from(error);
        assert_eq!(json.tipo, TipoErrorJson::Tecnico);
        assert_eq!(json.codigo, "error_tecnico");
        assert_eq!(json.mensaje, MENSAJE_ERROR_TECNICO);
    }
}
