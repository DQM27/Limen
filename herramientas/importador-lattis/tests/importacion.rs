//! La importación completa con los dobles en memoria y **datos inventados**
//! (nunca reales): cada regla de adaptación, cada rechazo, y que lo
//! importado se comporta bien con los casos de uso de la app.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use limen_aplicacion::casos_de_uso::ingresos::ComandoEntrada;
    use limen_aplicacion::puertos::{AccionAuditada, Consultas, RegistroAuditado};
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_composicion::Aplicacion;
    use limen_dominio::contratista::ContratistaId;
    use limen_dominio::gafete::{NumeroGafete, TipoGafete};
    use limen_dominio::medio::TipoMedio;
    use limen_importador_lattis::traduccion::OPERADOR_IMPORTADOR;
    use limen_importador_lattis::{Datos, Resumen, importar};
    use limen_infra_memoria::{AlmacenMemoria, RelojFijo};
    use uuid::Uuid;

    fn u(n: u128) -> String {
        Uuid::from_u128(n).to_string()
    }

    fn ahora() -> DateTime<Utc> {
        "2026-10-09T17:00:00Z".parse().unwrap()
    }

    /// Los catálogos inventados: empresas, contratistas, gafetes, proveedoras
    /// y personal KOF.
    fn catalogos() -> Datos {
        Datos {
            // `activa` no se lee: una empresa "inactiva" entra igual.
            empresas: Some(format!(
                "INSERT INTO empresas (id, nombre, activa) VALUES\n\
                 ('{}', 'ACME S.A.', 'true'),\n\
                 ('{}', 'OTRA EMPRESA S.A.', 'true'),\n\
                 ('{}', 'EMPRESA SIN TRABAJADORES', 'false'),\n\
                 ('{}', 'acme s.a.', 'true')\n;",
                u(1),
                u(2),
                u(3),
                u(4)
            )),
            // `es_personal_ruta` no se lee: esas personas son contratistas.
            contratistas: Some(format!(
                "INSERT INTO contratistas (id, identificacion, nombre, empresa_id, tipo_ingreso, \
                 fecha_vencimiento_praind, activo, es_personal_ruta) VALUES\n\
                 ('{c1}', '1-1111-1111', 'josé peña', '{e1}', 'PRAIND', '2099-01-01', 'true', 'false'),\n\
                 ('{c2}', '2-2222-2222', 'ANA ROJAS', '{e1}', 'PRAIND', '2020-01-01', 'true', 'false'),\n\
                 ('{c3}', '5-5555-5555', 'CARLOS BLOQUEADO', '{e1}', 'PRAIND', '2099-01-01', 'false', 'false'),\n\
                 ('{c4}', '6-6666-6666', 'LUIS DE OTRA EMPRESA', '{e2}', 'SWAT', NULL, 'true', 'false'),\n\
                 ('{c5}', '7-7777-7777', 'MARTA DE RUTA', '{e1}', 'IN_HOUSE', '2099-01-01', 'true', 'true'),\n\
                 ('{c6}', '1-1111-1111', 'OTRO NOMBRE', '{e1}', 'PRAIND', '2099-01-01', 'true', 'false'),\n\
                 ('{c7}', '8-8888-8888', 'PEDRO O''BRIEN 3', '{e1}', 'PRAIND', '2099-01-01', 'true', 'false'),\n\
                 ('{c8}', '9-9999-9999', 'SIN EMPRESA', '{e9}', 'PRAIND', '2099-01-01', 'true', 'false'),\n\
                 ('{c9}', '1-2222-3333', 'SIN FECHA', '{e1}', 'PRAIND', NULL, 'true', 'false')\n;",
                c1 = u(11),
                c2 = u(12),
                c3 = u(13),
                c4 = u(14),
                c5 = u(15),
                c6 = u(16),
                c7 = u(17),
                c8 = u(18),
                c9 = u(19),
                e1 = u(1),
                e2 = u(2),
                e9 = u(99)
            )),
            gafetes: Some(format!(
                "INSERT INTO gafetes (id, numero, tipo, estado) VALUES\n\
                 ('{}', '1', 'CONTRATISTA', 'DISPONIBLE'),\n\
                 ('{}', '2', 'CONTRATISTA', 'DISPONIBLE'),\n\
                 ('{}', '3', 'CONTRATISTA', 'DISPONIBLE'),\n\
                 ('{}', '1', 'PROVEEDOR', 'DISPONIBLE'),\n\
                 ('{}', '1', 'VISITA', 'DISPONIBLE'),\n\
                 ('{}', '1', 'PROVISIONAL_KOF', 'DISPONIBLE'),\n\
                 ('{}', '1', 'CONTRATISTA', 'DISPONIBLE')\n;",
                u(21),
                u(22),
                u(23),
                u(24),
                u(25),
                u(26),
                u(27)
            )),
            empresas_proveedor: Some(format!(
                "INSERT INTO empresas_proveedor (id, nombre) VALUES\n('{}', 'GAS ZETA')\n;",
                u(31)
            )),
            personal_kof: Some(format!(
                "INSERT INTO personal_kof (id, codigo_empleado, nombre) VALUES\n\
                 ('{}', '5040017', 'michael araya'),\n\
                 ('{}', '123', 'CODIGO CORTO'),\n\
                 ('{}', '5040017', 'CODIGO REPETIDO')\n;",
                u(41),
                u(42),
                u(43)
            )),
            ..Datos::default()
        }
    }

    /// Un volcado inventado que toca todas las reglas: los catálogos y los
    /// movimientos (ingresos y préstamos).
    fn datos() -> Datos {
        Datos {
            ingresos_contratista: Some(format!(
                "INSERT INTO ingresos_contratista (id, contratista_id, cedula, nombre, empresa_nombre, \
                 tipo_ingreso, medio_ingreso, gafete_numero, placa, hora_entrada, hora_salida, \
                 resultado_acceso, motivo_resultado, usuario_entrada, usuario_salida) VALUES\n\
                 ('{i1}', '{c1}', 'x', 'x', 'x', 'PRAIND', 'CAMINANDO', NULL, NULL, '2026-09-20 14:00:00+00', '2026-09-20 15:00:00+00', NULL, NULL, 'op', 'op'),\n\
                 ('{i2}', '{c1}', 'x', 'x', 'x', 'PRAIND', 'CAMINANDO', '2', NULL, '2026-10-09 13:00:00+00', NULL, NULL, NULL, 'op', NULL),\n\
                 ('{i3}', '{c2}', 'x', 'x', 'x', 'PRAIND', 'VEHICULO', NULL, NULL, '2026-09-21 10:00:00+00', '2026-09-21 11:00:00+00', NULL, NULL, 'op', 'op'),\n\
                 ('{i4}', '{c98}', 'x', 'x', 'x', 'PRAIND', 'CAMINANDO', NULL, NULL, '2026-09-21 10:00:00+00', NULL, NULL, NULL, 'op', NULL),\n\
                 ('{i5}', '{c1}', 'x', 'x', 'x', 'PRAIND', 'CAMINANDO', NULL, NULL, '2026-09-22 10:00:00+00', '2026-09-22 09:00:00+00', NULL, NULL, 'op', 'op'),\n\
                 ('{i6}', '{c5}', 'x', 'x', 'x', 'IN_HOUSE', 'CAMINANDO', '3', NULL, '2026-09-23 10:00:00+00', '2026-09-23 11:00:00+00', NULL, NULL, 'op', 'op'),\n\
                 ('{i7}', '{c1}', 'x', 'x', 'x', 'PRAIND', 'CAMINANDO', NULL, NULL, '2026-10-09 13:30:00+00', NULL, NULL, NULL, 'op', NULL),\n\
                 ('{i8}', '{c1}', 'x', 'x', 'x', 'PRAIND', 'BICICLETA', NULL, NULL, '2026-09-24 10:00:00+00', '2026-09-24 11:00:00+00', NULL, NULL, 'op', 'op')\n;",
                i1 = u(51),
                i2 = u(52),
                i3 = u(53),
                i4 = u(54),
                i5 = u(55),
                i6 = u(56),
                i7 = u(57),
                i8 = u(58),
                c1 = u(11),
                c2 = u(12),
                c5 = u(15),
                c98 = u(98)
            )),
            ingresos_proveedor: Some(format!(
                "INSERT INTO ingresos_proveedor (id, cedula, nombre, empresa_id, empresa_nombre, \
                 placa, gafete_numero, hora_entrada, hora_salida, usuario_entrada, usuario_salida) VALUES\n\
                 ('{}', '3-3333-3333', 'beto solís', NULL, 'gas zeta', 'abc-123', '1', '2026-10-09 12:00:00+00', NULL, 'op', NULL),\n\
                 ('{}', '3-4444-4444', 'ELENA', '{}', 'NO EXISTE', NULL, '1', '2026-09-25 08:00:00+00', '2026-09-25 09:00:00+00', 'op', 'op'),\n\
                 ('{}', '3-5555-5555', 'PABLO', NULL, 'GAS ZETA', '@@@ ñ*#', '1', '2026-09-26 08:00:00+00', '2026-09-26 09:00:00+00', 'op', 'op')\n;",
                u(61),
                u(62),
                u(77),
                u(63)
            )),
            ingresos_correo: Some(format!(
                "INSERT INTO ingresos_correo (id, cedula, nombre, motivo, placa, gafete_numero, \
                 hora_entrada, hora_salida, usuario_entrada, usuario_salida) VALUES\n\
                 ('{}', '4-4444-4444', 'luis mora', 'Entrevista con RH', NULL, '1', '2026-10-09 11:00:00+00', NULL, 'op', NULL)\n;",
                u(71)
            )),
            prestamos_kof: Some(format!(
                "INSERT INTO prestamos_kof (id, personal_id, codigo_empleado, gafete_numero, \
                 hora_entrega, hora_devolucion, usuario_entrega, usuario_devolucion) VALUES\n\
                 ('{}', '{p}', '5040017', '1', '2026-10-01 08:00:00+00', '2026-10-01 09:00:00+00', 'op', 'op'),\n\
                 ('{}', '{p}', '5040017', '1', '2026-10-09 10:00:00+00', NULL, 'op', NULL),\n\
                 ('{}', '{q}', '5040017', '1', '2026-10-02 08:00:00+00', NULL, 'op', NULL),\n\
                 ('{}', '{p}', '5040017', '1', '2026-10-03 08:00:00+00', '2026-10-03 09:00:00+00', 'op', 'op')\n;",
                u(81),
                u(82),
                u(83),
                u(84),
                p = u(41),
                q = u(999)
            )),
            ..catalogos()
        }
    }

    async fn importado() -> (AlmacenMemoria, Resumen) {
        let almacen = AlmacenMemoria::new();
        let resumen = importar(&almacen, &almacen.ids(), ahora(), &datos())
            .await
            .unwrap();
        (almacen, resumen)
    }

    fn motivos(resumen: &Resumen, tabla: &str) -> Vec<String> {
        let mut motivos: Vec<String> = resumen
            .tabla(tabla)
            .unwrap()
            .rechazos
            .iter()
            .map(|rechazo| rechazo.motivo.clone())
            .collect();
        motivos.sort();
        motivos
    }

    #[tokio::test]
    async fn las_empresas_no_desaparecen_pero_el_nombre_repetido_se_rechaza() {
        let (_, resumen) = importado().await;
        let tabla = resumen.tabla("empresas").unwrap();
        assert_eq!((tabla.leidas, tabla.cargadas), (4, 3), "{resumen}");
        assert_eq!(motivos(&resumen, "empresas"), ["nombre_repetido"]);
    }

    #[tokio::test]
    async fn los_contratistas_se_adaptan_y_lo_que_rompe_una_regla_se_rechaza() {
        let (_, resumen) = importado().await;
        let tabla = resumen.tabla("contratistas").unwrap();
        assert_eq!((tabla.leidas, tabla.cargadas), (9, 5), "{resumen}");
        assert_eq!(
            motivos(&resumen, "contratistas"),
            [
                "cedula_repetida",
                "empresa_no_existe",
                "nombre_invalido",
                "praind_sin_fecha"
            ]
        );
        assert_eq!(
            tabla.ajustes.get("praind_ficticio"),
            Some(&1),
            "el de tipo SWAT, sin PRAIND, recibió una fecha ficticia"
        );
    }

    #[tokio::test]
    async fn gafetes_proveedoras_y_personal_kof() {
        let (_, resumen) = importado().await;
        let gafetes = resumen.tabla("gafetes").unwrap();
        assert_eq!((gafetes.leidas, gafetes.cargadas), (7, 6), "{resumen}");
        assert_eq!(motivos(&resumen, "gafetes"), ["gafete_repetido"]);
        assert_eq!(resumen.tabla("empresas_proveedor").unwrap().cargadas, 1);
        let personal = resumen.tabla("personal_kof").unwrap();
        assert_eq!((personal.leidas, personal.cargadas), (3, 1), "{resumen}");
        assert!(
            motivos(&resumen, "personal_kof").contains(&"codigo_repetido".to_owned()),
            "{resumen}"
        );
    }

    #[tokio::test]
    async fn los_ingresos_se_cargan_con_sus_ajustes_y_se_rechaza_lo_imposible() {
        let (_, resumen) = importado().await;
        let tabla = resumen.tabla("ingresos_contratista").unwrap();
        assert_eq!((tabla.leidas, tabla.cargadas), (8, 4), "{resumen}");
        assert_eq!(
            motivos(&resumen, "ingresos_contratista"),
            [
                "contratista_no_importado",
                "medio_invalido",
                "salida_anterior_a_la_entrada",
                "ya_estaba_adentro_por_otra_via"
            ]
        );
        assert_eq!(
            tabla.ajustes.get("placa_inventada"),
            Some(&1),
            "el vehículo sin placa recibió una"
        );
        let proveedor = resumen.tabla("ingresos_proveedor").unwrap();
        assert_eq!((proveedor.leidas, proveedor.cargadas), (3, 2), "{resumen}");
        assert_eq!(
            proveedor.ajustes.get("placa_inventada"),
            Some(&1),
            "una placa que Limen no acepta también se reemplaza"
        );
        assert_eq!(
            motivos(&resumen, "ingresos_proveedor"),
            ["empresa_proveedora_no_importada"]
        );
        assert_eq!(resumen.tabla("ingresos_correo").unwrap().cargadas, 1);
        let prestamos = resumen.tabla("prestamos_kof").unwrap();
        assert_eq!(
            (prestamos.leidas, prestamos.cargadas),
            (4, 3),
            "un préstamo cerrado con id mayor que el abierto no debe chocar: {resumen}"
        );
        assert_eq!(
            motivos(&resumen, "prestamos_kof"),
            ["personal_kof_no_importado"]
        );
    }

    #[tokio::test]
    async fn los_ingresos_abiertos_dejan_a_la_persona_adentro_por_su_via() {
        let (almacen, _) = importado().await;
        let adentro = almacen.quienes_estan_adentro().await.unwrap();
        let mut vias: Vec<_> = adentro
            .iter()
            .map(|persona| {
                (
                    persona.ingreso.via().codigo(),
                    persona.gafete.map(NumeroGafete::valor),
                )
            })
            .collect();
        vias.sort_unstable();
        assert_eq!(
            vias,
            [
                ("CONTRATISTA", Some(2)),
                ("CORREO", Some(1)),
                ("KOF", Some(1)),
                ("PROVEEDOR", Some(1))
            ],
            "uno por cada vía, cada uno con su gafete"
        );
        let proveedor = adentro
            .iter()
            .find(|persona| persona.ingreso.via().codigo() == "PROVEEDOR")
            .unwrap();
        assert_eq!(proveedor.procedencia, "GAS ZETA", "enlazada por el nombre");
        assert_eq!(proveedor.desde.to_rfc3339(), "2026-10-09T12:00:00+00:00");
    }

    /// La app sobre lo importado, con el reloj después del último movimiento.
    fn app(
        almacen: &AlmacenMemoria,
    ) -> Aplicacion<AlmacenMemoria, RelojFijo, impl Clone + limen_aplicacion::puertos::GeneradorIds>
    {
        let reloj = RelojFijo::new(
            "2026-10-09T18:00:00Z".parse().unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        );
        Aplicacion::nueva(almacen, &reloj, &almacen.ids())
    }

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    fn entrada(contratista: u128) -> ComandoEntrada {
        ComandoEntrada {
            contratista: ContratistaId::desde_uuid(Uuid::from_u128(contratista)),
            medio: TipoMedio::APie,
            placa: None,
            gafete: None,
        }
    }

    #[tokio::test]
    async fn lo_importado_se_comporta_como_en_la_realidad_con_los_casos_de_uso() {
        let (almacen, _) = importado().await;
        let app = app(&almacen);

        let vencido = app.ingresos.entrada.ejecutar(&sesion(), &entrada(12)).await;
        assert_eq!(
            vencido.unwrap_err().para_interfaz().codigo,
            "praind_vencido",
            "existe, pero no entra (D4)"
        );
        let bloqueado = app.ingresos.entrada.ejecutar(&sesion(), &entrada(13)).await;
        assert_eq!(
            bloqueado.unwrap_err().para_interfaz().codigo,
            "sin_acceso",
            "el bloqueado existe y se le niega (D2)"
        );
        for (nombre, contratista) in [("el de tipo SWAT", 14), ("el IN HOUSE de ruta", 15)] {
            let entro = app
                .ingresos
                .entrada
                .ejecutar(&sesion(), &entrada(contratista))
                .await;
            assert!(entro.is_ok(), "{nombre} entra: {entro:?}");
        }
    }

    #[tokio::test]
    async fn la_busqueda_encuentra_lo_importado_normalizado() {
        let (almacen, _) = importado().await;
        let app = app(&almacen);
        let encontrados = app
            .contratistas
            .buscar
            .ejecutar("jose pe", 10)
            .await
            .unwrap();
        assert_eq!(encontrados.len(), 1);
        assert_eq!(encontrados[0].nombre().as_str(), "JOSE PEÑA");
        assert_eq!(encontrados[0].cedula().as_str(), "111111111");
        let vencidos = app.contratistas.praind_por_vencer.ejecutar().await.unwrap();
        assert_eq!(
            vencidos.len(),
            1,
            "el de PRAIND vencido aparece en la lista de por vencer"
        );
    }

    #[tokio::test]
    async fn los_gafetes_prestados_a_los_de_adentro_figuran_prestados() {
        let (almacen, _) = importado().await;
        let app = app(&almacen);
        let gafetes = app
            .gafetes
            .listar
            .ejecutar(TipoGafete::Contratista)
            .await
            .unwrap();
        let prestados: Vec<u32> = gafetes
            .iter()
            .filter(|resumen| resumen.prestado)
            .map(|resumen| resumen.gafete.numero().valor())
            .collect();
        assert_eq!(prestados, [2], "sólo el que tiene el ingreso abierto");
    }

    #[tokio::test]
    async fn cada_alta_queda_auditada_a_nombre_del_importador() {
        let (almacen, _) = importado().await;
        let app = app(&almacen);
        let historial = app
            .historial
            .ejecutar(RegistroAuditado::Contratista(ContratistaId::desde_uuid(
                Uuid::from_u128(11),
            )))
            .await
            .unwrap();
        assert_eq!(historial.len(), 1);
        assert_eq!(historial[0].accion, AccionAuditada::Alta);
        assert_eq!(historial[0].operador.uuid(), OPERADOR_IMPORTADOR);
    }

    #[tokio::test]
    async fn el_resumen_no_deja_ver_datos_de_personas() {
        let (_, resumen) = importado().await;
        let texto = resumen.to_string().to_uppercase();
        for dato in [
            "PEÑA",
            "ANA ROJAS",
            "1-1111",
            "111111111",
            "MICHAEL",
            "BETO",
            "5040017",
        ] {
            assert!(
                !texto.contains(dato),
                "el resumen no debe mostrar {dato}: {texto}"
            );
        }
        assert!(
            texto.contains("CEDULA_REPETIDA"),
            "pero sí los códigos: {texto}"
        );
    }

    #[tokio::test]
    async fn volver_a_importar_sobre_la_misma_base_no_mezcla_en_silencio() {
        let (almacen, _) = importado().await;
        let otra_vez = importar(&almacen, &almacen.ids(), ahora(), &datos()).await;
        assert!(otra_vez.is_err(), "choca con lo que ya hay y lo dice");
    }
}
