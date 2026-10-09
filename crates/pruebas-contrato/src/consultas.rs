//! Contrato de las consultas de lectura: quién está adentro y los
//! buscadores. Lo importante es que todos los adaptadores contesten igual
//! que la definición de referencia (`Criterio::coincide_con`) y en el mismo
//! orden.

use limen_aplicacion::puertos::{Consultas, FabricaUnidadDeTrabajo, IngresoAbierto};
use limen_aplicacion::puertos::{RepositorioEmpresas, RepositorioEmpresasProveedoras};
use limen_aplicacion::puertos::{
    RepositorioIngresos, RepositorioIngresosCorreo, RepositorioIngresosProveedor,
    RepositorioPersonalKof, UnidadDeTrabajo,
};
use limen_dominio::busqueda::{Criterio, relevantes};
use limen_dominio::cedula::Cedula;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::{Empresa, EmpresaId, NombreEmpresa};
use limen_dominio::empresa_proveedora::{EmpresaProveedora, EmpresaProveedoraId};
use limen_dominio::gafete::NumeroGafete;
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

fn nombres<T>(encontrados: &[T], nombre: impl Fn(&T) -> String) -> Vec<String> {
    encontrados.iter().map(nombre).collect()
}

// --- Quién está adentro ---

pub async fn quienes_estan_adentro_junta_las_tres_vias_del_mas_reciente_al_mas_antiguo<
    F: FabricaUnidadDeTrabajo + Consultas,
>(
    fabrica: F,
) {
    // La empresa 1 se llama ACME; el contratista 1 es ANA.
    sembrar(&fabrica, &[contratista(1, "111111111", "ANA")]).await;
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
    uow.confirmar().await.unwrap();

    let adentro = fabrica.quienes_estan_adentro().await.unwrap();
    let resumen: Vec<(String, String, String)> = adentro
        .iter()
        .map(|p| {
            (
                p.nombre.to_string(),
                p.procedencia.clone(),
                p.cedula.to_string(),
            )
        })
        .collect();
    assert_eq!(
        resumen,
        [
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
    assert_eq!(
        adentro[0].ingreso,
        IngresoAbierto::Proveedor(IngresoProveedorId::desde_uuid(Uuid::from_u128(6001)))
    );
    assert_eq!(
        adentro[1].ingreso,
        IngresoAbierto::Correo(IngresoCorreoId::desde_uuid(Uuid::from_u128(7001)))
    );
    assert_eq!(
        adentro[2].ingreso,
        IngresoAbierto::Contratista(IngresoId::desde_uuid(Uuid::from_u128(5001)))
    );
    assert_eq!(adentro[2].gafete, Some(NumeroGafete::nuevo(7).unwrap()));
    assert_eq!(
        adentro[2].medio,
        Medio::Vehiculo(Placa::nueva("ABC-123").unwrap())
    );
    assert_eq!(adentro[2].desde, marca("2026-10-09T08:00:00Z").en);
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
    assert_eq!(buscar("ñandu").await, ["666777888"]);
    // Cualquier orden, con una palabra del medio sin escribir.
    assert_eq!(buscar("sanches carlos").await, ["444555666"]);
    // Errores de tecleo: sólo se muestran si no hay algo mejor.
    assert_eq!(
        buscar("sanchez").await,
        ["777888999", "444555666"],
        "SOFIA SANCHEZ es exacta; CARLOS ... SANCHES, aproximada"
    );
    assert_eq!(buscar("hernandes").await, ["555666777"]);
    assert_eq!(
        buscar("mraia").await,
        ["555666777"],
        "letras cambiadas de lugar"
    );
    // Dentro del nombre.
    assert_eq!(buscar("ndez").await, ["555666777"]);
    // Nada.
    assert_eq!(buscar("zzz").await, Vec::<String>::new());
    assert_eq!(buscar("  ").await, Vec::<String>::new());
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
    assert!(!inactiva[0].activo(), "también devuelve a las inactivas");
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
