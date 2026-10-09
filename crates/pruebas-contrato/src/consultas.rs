//! Contrato de las consultas de lectura: quién está adentro y los
//! buscadores. Lo importante es que todos los adaptadores contesten igual
//! que la definición de referencia (`Criterio::coincide_con`) y en el mismo
//! orden.

use chrono::NaiveDate;
use limen_aplicacion::puertos::{
    AccionAuditada, Consultas, EntradaAuditoria, FabricaUnidadDeTrabajo, IngresoAbierto,
    RegistroAuditado, RegistroAuditoria,
};
use limen_aplicacion::puertos::{
    RepositorioEmpresas, RepositorioEmpresasProveedoras, RepositorioGafetes,
};
use limen_aplicacion::puertos::{
    RepositorioIngresos, RepositorioIngresosCorreo, RepositorioIngresosProveedor,
    RepositorioPersonalKof, RepositorioPrestamosKof, UnidadDeTrabajo,
};
use limen_dominio::auditoria::CambioCampo;
use limen_dominio::busqueda::{Criterio, relevantes};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::{Contratista, ContratistaGuardado};
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::{Deudor, EstadoGafete, Gafete, NumeroGafete, TipoGafete};
use limen_dominio::ingreso_contratista::{IngresoContratista, IngresoGuardado, IngresoId};
use limen_dominio::ingreso_correo::{
    IngresoCorreo, IngresoCorreoGuardado, IngresoCorreoId, Motivo,
};
use limen_dominio::ingreso_proveedor::{
    IngresoProveedor, IngresoProveedorGuardado, IngresoProveedorId,
};
use limen_dominio::medio::{Medio, Placa};
use limen_dominio::movimiento::Marca;
use limen_dominio::nombre::NombrePersona;
use limen_dominio::operador::OperadorId;
use limen_dominio::personal_kof::{CodigoEmpleado, PersonalKof, PersonalKofId};
use limen_dominio::prestamo_kof::{PrestamoKof, PrestamoKofGuardado, PrestamoKofId};
use limen_dominio::tipo_ingreso::TipoIngreso;
use limen_dominio::visitante::Visitante;
use uuid::Uuid;

use super::{contratista, id_contratista, sembrar};

fn marca(texto: &str) -> Marca {
    Marca {
        en: texto.parse().unwrap(),
        operador: OperadorId::desde_uuid(Uuid::from_u128(900)),
    }
}

fn cedula(texto: &str) -> Cedula {
    Cedula::normalizar(texto).unwrap()
}

// --- Quién está adentro ---

/// Un contratista (08:00), una visita por correo (09:00), un proveedor
/// (10:00) y una persona del KOF con gafete provisional (11:00), todos
/// adentro.
async fn sembrar_las_cuatro_vias<F: FabricaUnidadDeTrabajo>(fabrica: &F) {
    // La empresa 1 se llama ACME; el contratista 1 es ANA.
    sembrar(fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let proveedora = EmpresaProveedoraId::desde_uuid(Uuid::from_u128(3001));
    let mut uow = fabrica.nueva();
    uow.empresas_proveedoras()
        .guardar(&EmpresaProveedora::restaurar(
            proveedora,
            NombreEmpresa::nuevo("GAS ZETA").unwrap(),
        ));
    uow.confirmar().await.unwrap();

    let mut uow = fabrica.nueva();
    // 08:00 contratista en carro con gafete 7.
    uow.ingresos()
        .guardar(&IngresoContratista::restaurar(IngresoGuardado {
            id: IngresoId::desde_uuid(Uuid::from_u128(5001)),
            contratista: id_contratista(1),
            cedula: cedula("111111111"),
            medio: Medio::Vehiculo(Placa::nueva("ABC-123").unwrap()),
            gafete: Some(NumeroGafete::nuevo(7).unwrap()),
            entrada: marca("2026-10-09T08:00:00Z"),
            salida: None,
        }));
    // 10:00 proveedor a pie.
    uow.ingresos_proveedor()
        .guardar(&IngresoProveedor::restaurar(IngresoProveedorGuardado {
            id: IngresoProveedorId::desde_uuid(Uuid::from_u128(6001)),
            visitante: Visitante::restaurar(
                cedula("222222222"),
                NombrePersona::nuevo("BETO SOLÍS").unwrap(),
            ),
            empresa: proveedora,
            medio: Medio::APie,
            gafete: NumeroGafete::nuevo(4).unwrap(),
            entrada: marca("2026-10-09T10:00:00Z"),
            salida: None,
        }));
    // 09:00 visita por correo.
    uow.ingresos_correo()
        .guardar(&IngresoCorreo::restaurar(IngresoCorreoGuardado {
            id: IngresoCorreoId::desde_uuid(Uuid::from_u128(7001)),
            visitante: Visitante::restaurar(
                cedula("333333333"),
                NombrePersona::nuevo("LUIS MORA").unwrap(),
            ),
            motivo: Motivo::nuevo("Entrevista con RH").unwrap(),
            medio: Medio::APie,
            gafete: NumeroGafete::nuevo(2).unwrap(),
            entrada: marca("2026-10-09T09:00:00Z"),
            salida: None,
        }));
    // 11:00 personal KOF con el gafete provisional 3.
    uow.prestamos_kof()
        .anotar_entrega(&PrestamoKof::restaurar(PrestamoKofGuardado {
            id: PrestamoKofId::desde_uuid(Uuid::from_u128(8001)),
            personal: PersonalKofId::desde_uuid(Uuid::from_u128(4001)),
            codigo: CodigoEmpleado::nuevo("5040017").unwrap(),
            nombre: NombrePersona::nuevo("MICHAEL ARAYA").unwrap(),
            gafete: NumeroGafete::nuevo(3).unwrap(),
            entrega: marca("2026-10-09T11:00:00Z"),
            devolucion: None,
        }));
    uow.confirmar().await.unwrap();
}

pub async fn quienes_estan_adentro_junta_las_cuatro_vias_del_mas_reciente_al_mas_antiguo<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    sembrar_las_cuatro_vias(&fabrica).await;

    let adentro = fabrica.quienes_estan_adentro().await.unwrap();
    let resumen: Vec<(String, String, String)> = adentro
        .iter()
        .map(|p| {
            (
                p.nombre.to_string(),
                p.procedencia.clone(),
                p.identidad.to_string(),
            )
        })
        .collect();
    assert_eq!(
        resumen,
        [
            (
                "MICHAEL ARAYA".into(),
                "Personal KOF".into(),
                "5040017".into()
            ),
            ("BETO SOLIS".into(), "GAS ZETA".into(), "222222222".into()),
            (
                "LUIS MORA".into(),
                "Entrevista con RH".into(),
                "333333333".into()
            ),
            ("ANA".into(), "ACME".into(), "111111111".into()),
        ],
        "del más reciente al más antiguo, con la empresa o el motivo"
    );
    let [kof, proveedor, visita, contratista] = <[_; 4]>::try_from(adentro).unwrap();
    assert_eq!(
        kof.ingreso,
        IngresoAbierto::Kof(PrestamoKofId::desde_uuid(Uuid::from_u128(8001))),
        "el préstamo abierto del KOF, para devolver el gafete"
    );
    assert_eq!(kof.medio, None, "el personal KOF no registra cómo llegó");
    assert_eq!(
        kof.gafete,
        Some(NumeroGafete::nuevo(3).unwrap()),
        "su gafete provisional"
    );
    assert_eq!(
        proveedor.ingreso,
        IngresoAbierto::Proveedor(IngresoProveedorId::desde_uuid(Uuid::from_u128(6001))),
        "el ingreso abierto del proveedor, para registrar su salida"
    );
    assert_eq!(
        visita.ingreso,
        IngresoAbierto::Correo(IngresoCorreoId::desde_uuid(Uuid::from_u128(7001))),
        "el ingreso abierto de la visita"
    );
    assert_eq!(
        contratista.ingreso,
        IngresoAbierto::Contratista(IngresoId::desde_uuid(Uuid::from_u128(5001))),
        "el ingreso abierto del contratista"
    );
    assert_eq!(
        contratista.gafete,
        Some(NumeroGafete::nuevo(7).unwrap()),
        "su gafete"
    );
    assert_eq!(
        contratista.medio,
        Some(Medio::Vehiculo(Placa::nueva("ABC-123").unwrap())),
        "llegó en carro"
    );
    assert_eq!(
        contratista.desde,
        marca("2026-10-09T08:00:00Z").en,
        "desde cuándo"
    );
}

pub async fn quienes_estan_adentro_ignora_a_quienes_ya_salieron<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    assert!(
        fabrica.quienes_estan_adentro().await.unwrap().is_empty(),
        "una base vacía no tiene a nadie adentro"
    );
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let mut uow = fabrica.nueva();
    uow.ingresos()
        .guardar(&IngresoContratista::restaurar(IngresoGuardado {
            id: IngresoId::desde_uuid(Uuid::from_u128(5001)),
            contratista: id_contratista(1),
            cedula: cedula("111111111"),
            medio: Medio::APie,
            gafete: None,
            entrada: marca("2026-10-09T08:00:00Z"),
            salida: Some(marca("2026-10-09T09:00:00Z")),
        }));
    uow.confirmar().await.unwrap();
    assert!(
        fabrica.quienes_estan_adentro().await.unwrap().is_empty(),
        "ya salió"
    );
}

// --- Buscadores ---

/// Personas con nombres parecidos entre sí, con Ñ, con errores típicos y
/// con cédulas que se contienen unas a otras.
fn personas() -> Vec<Contratista> {
    vec![
        contratista(1, "111111111", "JOSE PEÑA"),
        contratista(2, "111222333", "PEDRO JOSE"),
        contratista(3, "222333444", "JOSE MORA"),
        contratista(4, "333444555", "ANA PENA"),
        contratista(5, "111111112", "JOSE PEÑA"),
        contratista(6, "444555666", "CARLOS MAURICIO SANCHES"),
        contratista(7, "555666777", "MARIA HERNANDEZ"),
        contratista(8, "666777888", "ÑANDU RIOS"),
        contratista(9, "777888999", "SOFIA SANCHEZ"),
        contratista(10, "888999000", "LUIS MARIO"),
        contratista(11, "911112223", "MARIO LUIS ZAMORA"),
        contratista(12, "999111111", "NOEL ARAYA"),
    ]
}

const BUSQUEDAS: [&str; 34] = [
    "",
    "   ",
    "-",
    "1-1111",
    "111",
    "1111",
    "11111",
    "0111111111",
    "111111111",
    "2",
    "9",
    "josé",
    "jose pe",
    "j p",
    "peña",
    "pena",
    "PEÑA",
    "nandu",
    "ñandu",
    "carlos sanches",
    "carlos sanchez",
    "sanchez",
    "sanches",
    "hernandes",
    "ndez",
    "mari",
    "mario luis",
    "luis mario",
    "jsoe",
    "mraia",
    "zzz",
    "a",
    "an",
    "s",
];

fn cedulas(encontrados: &[Contratista]) -> Vec<String> {
    encontrados.iter().map(|p| p.cedula().to_string()).collect()
}

/// La prueba central: lo que devuelve el adaptador (que trae candidatos con
/// índices y por etapas) es exactamente lo que daría recorrer todo y
/// aplicar la regla del dominio, para cualquier texto y cualquier límite.
pub async fn buscar_contratistas_da_lo_mismo_que_recorrer_todo_con_la_regla_del_dominio<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    sembrar(&fabrica, &personas()).await;
    for texto in BUSQUEDAS {
        let criterio = Criterio::desde_texto(texto);
        for limite in [1, 2, 3, 5, 50] {
            let esperado = relevantes(&criterio, personas(), limite);
            let obtenido = fabrica
                .buscar_contratistas(&criterio, limite)
                .await
                .unwrap();
            assert_eq!(
                cedulas(&obtenido),
                cedulas(&esperado),
                "buscando {texto:?} con límite {limite}"
            );
        }
    }
}

pub async fn buscar_contratistas_tolera_tildes_enie_orden_y_errores_de_tecleo<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    sembrar(&fabrica, &personas()).await;
    let buscar = |texto: &'static str| {
        let fabrica = &fabrica;
        async move {
            cedulas(
                &fabrica
                    .buscar_contratistas(&Criterio::desde_texto(texto), 50)
                    .await
                    .unwrap(),
            )
        }
    };
    // La Ñ se encuentra escribiendo N, y al revés.
    assert_eq!(
        buscar("pena").await,
        ["333444555", "111111111", "111111112"],
        "palabras completas, por nombre y luego por cédula (ANA PENA, JOSE PEÑA, JOSE PEÑA)"
    );
    assert_eq!(buscar("ñandu").await, ["666777888"], "Ñ escrita con Ñ");
    // Cualquier orden, con una palabra del medio sin escribir.
    assert_eq!(
        buscar("sanches carlos").await,
        ["444555666"],
        "el nombre del medio no estorba"
    );
    // Errores de tecleo: sólo se muestran si no hay algo mejor.
    assert_eq!(
        buscar("sanchez").await,
        ["777888999", "444555666"],
        "SOFIA SANCHEZ es exacta; CARLOS ... SANCHES, aproximada"
    );
    assert_eq!(
        buscar("hernandes").await,
        ["555666777"],
        "una letra distinta"
    );
    assert_eq!(
        buscar("mraia").await,
        ["555666777"],
        "letras cambiadas de lugar"
    );
    // Dentro del nombre.
    assert_eq!(buscar("ndez").await, ["555666777"], "dentro de una palabra");
    // Nada.
    assert_eq!(buscar("zzz").await, Vec::<String>::new(), "nada se parece");
    assert_eq!(
        buscar("  ").await,
        Vec::<String>::new(),
        "espacios no buscan nada"
    );
}

pub async fn buscar_contratistas_por_cedula_ordena_exacta_primero<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    sembrar(&fabrica, &personas()).await;
    let criterio = Criterio::desde_texto("111111111");
    assert_eq!(
        cedulas(&fabrica.buscar_contratistas(&criterio, 50).await.unwrap()),
        ["111111111"],
        "sólo la exacta: ninguna otra empieza igual ni la contiene"
    );
    let criterio = Criterio::desde_texto("1-1111");
    assert_eq!(
        cedulas(&fabrica.buscar_contratistas(&criterio, 50).await.unwrap()),
        ["111111111", "111111112", "999111111"],
        "las que empiezan igual (por nombre y cédula) y luego las que lo contienen"
    );
}

pub async fn buscar_contratistas_respeta_el_limite_y_el_criterio_vacio<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    sembrar(&fabrica, &personas()).await;
    let criterio = Criterio::desde_texto("jose");
    let dos = fabrica.buscar_contratistas(&criterio, 2).await.unwrap();
    assert_eq!(
        cedulas(&dos),
        ["222333444", "111111111"],
        "el límite corta después de ordenar"
    );
    assert_eq!(
        cedulas(
            &fabrica
                .buscar_contratistas(&Criterio::Vacio, 50)
                .await
                .unwrap()
        ),
        Vec::<String>::new(),
        "un criterio vacío no busca nada"
    );
}

pub async fn buscar_personal_kof_por_codigo_o_nombre<F: FabricaUnidadDeTrabajo + Consultas>(
    fabrica: F,
) {
    let persona = |n: u128, codigo: &str, nombre: &str, activa: bool| {
        PersonalKof::restaurar(
            PersonalKofId::desde_uuid(Uuid::from_u128(4000 + n)),
            CodigoEmpleado::nuevo(codigo).unwrap(),
            NombrePersona::nuevo(nombre).unwrap(),
            activa,
        )
    };
    let todas = [
        persona(1, "5040017", "MICHAEL ARAYA", true),
        persona(2, "5040018", "ANA MORA", false),
        persona(3, "6000001", "MARIO ARAYA", true),
        persona(4, "7050404", "NOEMI PEÑA", true),
    ];
    let mut uow = fabrica.nueva();
    for p in &todas {
        uow.personal_kof().guardar(p);
    }
    uow.confirmar().await.unwrap();

    for texto in [
        "5040", "5040017", "504", "araya", "arya", "ana", "pena", "0404", "", "9",
    ] {
        let criterio = Criterio::desde_texto(texto);
        for limite in [1, 2, 50] {
            let esperado = relevantes(&criterio, todas.to_vec(), limite);
            let obtenido = fabrica
                .buscar_personal_kof(&criterio, limite)
                .await
                .unwrap();
            let codigos = |l: &[PersonalKof]| -> Vec<String> {
                l.iter().map(|p| p.codigo().to_string()).collect()
            };
            assert_eq!(
                codigos(&obtenido),
                codigos(&esperado),
                "buscando {texto:?} con límite {limite}"
            );
        }
    }
    let inactiva = fabrica
        .buscar_personal_kof(&Criterio::desde_texto("ana"), 50)
        .await
        .unwrap();
    let [ana] = <[_; 1]>::try_from(inactiva).unwrap();
    assert!(!ana.activo(), "también devuelve a las inactivas");
}

pub async fn buscar_empresas_por_nombre_con_numeros_y_signos<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    let nombres = [
        "ACME S.A.",
        "3M COSTA RICA",
        "CONSTRUCTORA PEÑA Y HIJOS",
        "GAS ZETA",
        "AIRES ACONDICIONADOS DEL ESTE",
        "ELECTRO-SERVICIOS",
    ];
    let empresas: Vec<Empresa> = nombres
        .iter()
        .enumerate()
        .map(|(n, nombre)| {
            Empresa::restaurar(
                EmpresaId::desde_uuid(Uuid::from_u128(9000 + n as u128)),
                NombreEmpresa::nuevo(nombre).unwrap(),
            )
        })
        .collect();
    let proveedoras: Vec<EmpresaProveedora> = nombres
        .iter()
        .enumerate()
        .map(|(n, nombre)| {
            EmpresaProveedora::restaurar(
                EmpresaProveedoraId::desde_uuid(Uuid::from_u128(9500 + n as u128)),
                NombreEmpresa::nuevo(nombre).unwrap(),
            )
        })
        .collect();
    let mut uow = fabrica.nueva();
    for e in &empresas {
        uow.empresas().guardar(e);
    }
    for e in &proveedoras {
        uow.empresas_proveedoras().guardar(e);
    }
    uow.confirmar().await.unwrap();

    for texto in [
        "acme",
        "s.a.",
        "3m",
        "3",
        "pena",
        "peña hijos",
        "gaz",
        "gas zeta",
        "zeta gas",
        "ndic",
        "acondicionados",
        "electro",
        "servicios",
        "electro-servicios",
        "aires este",
        "acne",
        "construtora",
        "-",
        "",
        "zzzz",
    ] {
        let criterio = Criterio::de_nombre(texto);
        for limite in [1, 3, 50] {
            let nombres_de = |l: &[Empresa]| -> Vec<String> {
                l.iter().map(|e| e.nombre().to_string()).collect()
            };
            let esperado = relevantes(&criterio, empresas.clone(), limite);
            let obtenido = fabrica.buscar_empresas(&criterio, limite).await.unwrap();
            assert_eq!(
                nombres_de(&obtenido),
                nombres_de(&esperado),
                "empresas: buscando {texto:?} con límite {limite}"
            );
            let esperado = relevantes(&criterio, proveedoras.clone(), limite);
            let obtenido = fabrica
                .buscar_empresas_proveedoras(&criterio, limite)
                .await
                .unwrap();
            let nombres_prov = |l: &[EmpresaProveedora]| -> Vec<String> {
                l.iter().map(|e| e.nombre().to_string()).collect()
            };
            assert_eq!(
                nombres_prov(&obtenido),
                nombres_prov(&esperado),
                "proveedoras: buscando {texto:?} con límite {limite}"
            );
        }
    }
}

// --- Historial de cambios ---

fn entrada_de_historial(
    n: u128,
    registro: RegistroAuditado,
    accion: AccionAuditada,
    campo: &'static str,
    antes: &str,
    despues: &str,
) -> EntradaAuditoria {
    EntradaAuditoria {
        id_entrada: Uuid::from_u128(n),
        registro,
        accion,
        cambios: vec![CambioCampo {
            campo,
            antes: antes.to_owned(),
            despues: despues.to_owned(),
        }],
        operador: OperadorId::desde_uuid(Uuid::from_u128(900)),
        en: marca("2026-10-09T10:00:00Z").en,
    }
}

pub async fn el_historial_va_en_orden_y_solo_trae_el_registro_pedido<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    let ana = RegistroAuditado::Contratista(id_contratista(1));
    let beto = RegistroAuditado::Contratista(id_contratista(2));
    let gafete = RegistroAuditado::Gafete(TipoGafete::Contratista, NumeroGafete::nuevo(7).unwrap());
    let mut uow = fabrica.nueva();
    // Se anotan desordenadas: el orden lo da el ID de cada entrada.
    for entrada in [
        entrada_de_historial(
            30,
            ana,
            AccionAuditada::Edicion,
            "nombre",
            "ANA",
            "ANA MORA",
        ),
        entrada_de_historial(10, ana, AccionAuditada::Alta, "nombre", "", "ANA"),
        entrada_de_historial(20, beto, AccionAuditada::Alta, "nombre", "", "BETO"),
        entrada_de_historial(25, gafete, AccionAuditada::Alta, "estado", "", "DISPONIBLE"),
        entrada_de_historial(
            40,
            ana,
            AccionAuditada::Edicion,
            "tiene_acceso",
            "true",
            "false",
        ),
    ] {
        uow.auditoria().anotar(entrada);
    }
    uow.confirmar().await.unwrap();

    let historial = fabrica.historial_de(ana).await.unwrap();
    let resumen: Vec<(AccionAuditada, &str, &str, &str)> = historial
        .iter()
        .map(|e| {
            let cambio = e.cambios.first().unwrap();
            (
                e.accion,
                cambio.campo.as_str(),
                cambio.antes.as_str(),
                cambio.despues.as_str(),
            )
        })
        .collect();
    assert_eq!(
        resumen,
        [
            (AccionAuditada::Alta, "nombre", "", "ANA"),
            (AccionAuditada::Edicion, "nombre", "ANA", "ANA MORA"),
            (AccionAuditada::Edicion, "tiene_acceso", "true", "false"),
        ],
        "del más antiguo al más reciente, sólo de Ana"
    );
    let [primera, ..] = <[_; 3]>::try_from(historial).unwrap();
    assert_eq!(
        primera.operador,
        OperadorId::desde_uuid(Uuid::from_u128(900)),
        "quién"
    );
    assert_eq!(primera.en, marca("2026-10-09T10:00:00Z").en, "cuándo");

    assert_eq!(
        fabrica.historial_de(gafete).await.unwrap().len(),
        1,
        "un gafete se identifica por tipo y número"
    );
    let otro = RegistroAuditado::Contratista(id_contratista(9));
    assert_eq!(
        fabrica.historial_de(otro).await.unwrap(),
        Vec::new(),
        "un registro sin cambios no tiene historial"
    );
}

// --- Listado de gafetes ---

pub async fn listar_gafetes_trae_los_del_tipo_con_su_estado_y_si_estan_prestados<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
    let numero = |n: u32| NumeroGafete::nuevo(n).unwrap();
    let disponible = |tipo: TipoGafete, n: u32| {
        Gafete::restaurar(tipo, numero(n), EstadoGafete::Disponible, None)
    };
    let perdido = Gafete::restaurar(
        TipoGafete::Contratista,
        numero(3),
        EstadoGafete::Perdido,
        Some(Deudor::Contratista(id_contratista(1))),
    );
    let mut uow = fabrica.nueva();
    for gafete in [
        disponible(TipoGafete::Contratista, 10),
        disponible(TipoGafete::Contratista, 2),
        perdido.clone(),
        disponible(TipoGafete::Visita, 2),
    ] {
        uow.gafetes().agregar(&gafete);
    }
    // El 2 de contratista está prestado; el 2 de visita, no.
    uow.gafetes().anotar_prestamo(
        TipoGafete::Contratista,
        numero(2),
        marca("2026-10-09T08:00:00Z").en,
    );
    uow.confirmar().await.unwrap();

    let contratistas = fabrica
        .listar_gafetes(TipoGafete::Contratista)
        .await
        .unwrap();
    let resumen: Vec<(u32, bool)> = contratistas
        .iter()
        .map(|r| (r.gafete.numero().valor(), r.prestado))
        .collect();
    assert_eq!(
        resumen,
        [(2, true), (3, false), (10, false)],
        "por número (no por texto) y con el préstamo del tipo correcto"
    );
    assert_eq!(
        contratistas.get(1).map(|r| r.gafete.clone()),
        Some(perdido),
        "el perdido conserva su deudor"
    );
    let visitas = fabrica.listar_gafetes(TipoGafete::Visita).await.unwrap();
    assert_eq!(
        visitas
            .iter()
            .map(|r| (r.gafete.numero().valor(), r.prestado))
            .collect::<Vec<_>>(),
        [(2, false)],
        "el préstamo del 2 de contratista no cuenta para el 2 de visita"
    );
    assert_eq!(
        fabrica
            .listar_gafetes(TipoGafete::Proveedor)
            .await
            .unwrap()
            .len(),
        0,
        "un tipo sin gafetes"
    );
}

// --- PRAIND por vencer ---

pub async fn contratistas_con_praind_hasta_filtra_ordena_y_respeta_el_limite<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    let fecha = |texto: &str| -> NaiveDate { texto.parse().unwrap() };
    let con = |n: u128, cedula: &str, nombre: &str, vence: &str, acceso: bool| {
        Contratista::restaurar(ContratistaGuardado {
            id: id_contratista(n),
            cedula: Cedula::normalizar(cedula).unwrap(),
            nombre: NombrePersona::nuevo(nombre).unwrap(),
            empresa: super::id_empresa(1),
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: fecha(vence),
            tiene_acceso: acceso,
        })
    };
    sembrar(
        &fabrica,
        &[
            con(1, "111111111", "ANA", "2026-10-01", true),
            con(2, "222222222", "BETO", "2026-10-20", true),
            con(3, "333333333", "ALBERTO", "2026-10-20", true),
            con(4, "444444444", "CARLOS", "2026-11-08", true),
            con(5, "555555555", "DIEGO", "2026-11-09", true),
            con(6, "666666666", "ELENA", "2026-10-05", false),
        ],
    )
    .await;

    let hasta = fecha("2026-11-08");
    let por_vencer = fabrica
        .contratistas_con_praind_hasta(hasta, 50)
        .await
        .unwrap();
    assert_eq!(
        cedulas(&por_vencer),
        ["111111111", "333333333", "222222222", "444444444"],
        "el vencido primero; el mismo día por nombre; el último día incluido; \
         sin DIEGO (después) ni ELENA (sin acceso)"
    );
    let dos = fabrica
        .contratistas_con_praind_hasta(hasta, 2)
        .await
        .unwrap();
    assert_eq!(
        cedulas(&dos),
        ["111111111", "333333333"],
        "el límite corta después de ordenar"
    );
    assert_eq!(
        cedulas(
            &fabrica
                .contratistas_con_praind_hasta(fecha("2026-09-30"), 50)
                .await
                .unwrap()
        ),
        Vec::<String>::new(),
        "nadie venció antes"
    );
}
