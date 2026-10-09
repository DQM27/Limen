//! El lector del volcado: valores, `NULL`, comillas, saltos de línea y errores.

#[cfg(test)]
mod tests {
    use limen_importador_lattis::lectura::{ErrorLectura, Tabla};

    #[test]
    fn lee_columnas_valores_y_null() {
        let tabla = Tabla::leer(
            "INSERT INTO t (a, b, c) VALUES\n('uno', NULL, 'tres'),\n('cuatro', 'cinco', NULL)\n;",
        )
        .unwrap();
        assert_eq!(tabla.len(), 2);
        let filas: Vec<_> = tabla.filas().collect();
        assert_eq!(filas[0].texto("a").unwrap(), "uno");
        assert_eq!(filas[0].opcional("b").unwrap(), None, "NULL");
        assert_eq!(filas[1].opcional("c").unwrap(), None);
        assert_eq!(filas[1].texto("b").unwrap(), "cinco");
    }

    #[test]
    fn las_comillas_se_escapan_duplicadas_y_el_texto_puede_traer_saltos_de_linea() {
        let tabla = Tabla::leer(
            "INSERT INTO t (a, b) VALUES ('O''BRIEN', 'línea 1\nlínea 2, (con) ''comas''');",
        )
        .unwrap();
        let fila = tabla.filas().next().unwrap();
        assert_eq!(fila.texto("a").unwrap(), "O'BRIEN");
        assert_eq!(fila.texto("b").unwrap(), "línea 1\nlínea 2, (con) 'comas'");
    }

    #[test]
    fn acepta_tildes_y_enes() {
        let tabla = Tabla::leer("INSERT INTO t (nombre) VALUES ('JOSÉ PEÑA');").unwrap();
        assert_eq!(
            tabla.filas().next().unwrap().texto("nombre").unwrap(),
            "JOSÉ PEÑA"
        );
    }

    #[test]
    fn una_tabla_sin_filas_esta_vacia() {
        let tabla = Tabla::leer("INSERT INTO t (a) VALUES\n;").unwrap();
        assert!(tabla.is_empty());
    }

    #[test]
    fn pedir_un_null_como_texto_o_una_columna_que_no_existe_es_un_error() {
        let tabla = Tabla::leer("INSERT INTO t (a) VALUES (NULL);").unwrap();
        let fila = tabla.filas().next().unwrap();
        assert_eq!(fila.texto("a"), Err(ErrorLectura::ValorNulo("a".into())));
        assert_eq!(
            fila.texto("x"),
            Err(ErrorLectura::ColumnaFaltante("x".into()))
        );
    }

    #[test]
    fn los_formatos_danados_se_rechazan_con_la_fila() {
        assert_eq!(
            Tabla::leer("INSERT INTO t (a) (1);"),
            Err(ErrorLectura::SinValores)
        );
        assert_eq!(
            Tabla::leer("INSERT INTO t VALUES ('x');"),
            Err(ErrorLectura::SinColumnas)
        );
        assert_eq!(
            Tabla::leer("INSERT INTO t (a) VALUES ('x'), (algo);"),
            Err(ErrorLectura::Formato(2))
        );
        assert_eq!(
            Tabla::leer("INSERT INTO t (a) VALUES ('sin cerrar);"),
            Err(ErrorLectura::Formato(1))
        );
        assert_eq!(
            Tabla::leer("INSERT INTO t (a, b) VALUES ('x');"),
            Err(ErrorLectura::Anchura {
                fila: 1,
                tiene: 1,
                esperados: 2
            })
        );
    }
}
