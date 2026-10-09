//! Pruebas de los casos de uso contra los dobles en memoria: qué se lee,
//! qué decide el dominio, qué se confirma y qué se audita, sin base de
//! datos.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use limen_aplicacion::casos_de_uso::contratistas::{
        ComandoContratista, ConsultarContratista, EditarContratista, RegistrarContratista,
    };
    use limen_aplicacion::casos_de_uso::empresas::{RegistrarEmpresa, RenombrarEmpresa};
    use limen_aplicacion::errores::{ErrorCaso, MENSAJE_ERROR_TECNICO, TipoError};
    use limen_aplicacion::puertos::{
        AccionAuditada, EntidadAuditada, ErrorPersistencia, Restriccion,
    };
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::acceso::{MotivoDenegacion, ResultadoAcceso};
    use limen_dominio::contratista::{ContratistaId, ErrorContratista};
    use limen_dominio::empresa::{Empresa, EmpresaId, ErrorEmpresa, NombreEmpresa};
    use limen_dominio::tipo_ingreso::TipoIngreso;
    use limen_infra_memoria::{AlmacenMemoria, IdsSecuenciales, RelojFijo};
    use uuid::Uuid;

    // --- Preparación común ---

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    fn instante(dia: &str) -> DateTime<Utc> {
        fecha(dia).and_hms_opt(15, 0, 0).unwrap().and_utc()
    }

    /// Reloj detenido en `dia` a las 15:00 UTC.
    fn reloj_en(dia: &str) -> RelojFijo {
        RelojFijo::new(instante(dia), fecha(dia))
    }

    const HOY: &str = "2026-10-09";

    fn reloj() -> RelojFijo {
        reloj_en(HOY)
    }

    fn sesion() -> Sesion {
        Sesion::nueva(OperadorId::desde_uuid(Uuid::from_u128(900)))
    }

    /// Almacén con la empresa ACME ya registrada.
    fn almacen_con_empresa() -> (AlmacenMemoria, EmpresaId) {
        let almacen = AlmacenMemoria::new();
        let id = EmpresaId::desde_uuid(Uuid::from_u128(500));
        almacen.sembrar_empresa(Empresa::restaurar(
            id,
            NombreEmpresa::nuevo("ACME").unwrap(),
        ));
        (almacen, id)
    }

    fn comando(empresa: EmpresaId) -> ComandoContratista {
        ComandoContratista {
            cedula: "1-1111-1111".into(),
            nombre: "  josé  peña ".into(),
            empresa,
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: fecha("2027-01-01"),
            tiene_acceso: true,
        }
    }

    fn registrar(
        almacen: &AlmacenMemoria,
    ) -> RegistrarContratista<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        RegistrarContratista::new(almacen.clone(), reloj(), almacen.ids())
    }

    fn editar(almacen: &AlmacenMemoria) -> EditarContratista<AlmacenMemoria, RelojFijo> {
        EditarContratista::new(almacen.clone(), reloj())
    }

    /// Registra el contratista de `comando` y devuelve su ID.
    async fn con_contratista(almacen: &AlmacenMemoria, empresa: EmpresaId) -> ContratistaId {
        registrar(almacen)
            .ejecutar(&sesion(), &comando(empresa))
            .await
            .unwrap()
    }

    // --- Registrar contratista ---

    #[tokio::test]
    async fn registra_normaliza_y_audita_el_alta() {
        let (almacen, empresa) = almacen_con_empresa();
        let id = con_contratista(&almacen, empresa).await;

        let guardados = almacen.contratistas();
        assert_eq!(guardados.len(), 1);
        assert_eq!(guardados[0].id(), id);
        assert_eq!(guardados[0].cedula().as_str(), "111111111");
        assert_eq!(guardados[0].nombre().as_str(), "JOSE PEÑA");

        let auditoria = almacen.auditoria();
        assert_eq!(auditoria.len(), 1, "un registro por operación");
        let alta = &auditoria[0];
        assert_eq!(alta.entidad, EntidadAuditada::Contratista);
        assert_eq!(alta.accion, AccionAuditada::Alta);
        assert_eq!(alta.id, id.uuid());
        assert_eq!(alta.operador, sesion().operador(), "quién lo hizo");
        assert_eq!(alta.en, instante(HOY), "cuándo, según el reloj");
        assert_eq!(alta.cambios.len(), 6, "todos los campos del contratista");
        assert_eq!(almacen.confirmaciones(), 1, "una sola transacción");
    }

    #[tokio::test]
    async fn una_cedula_invalida_se_rechaza_sin_tocar_la_base() {
        let (almacen, empresa) = almacen_con_empresa();
        almacen.fallar_lecturas(ErrorPersistencia::Tecnica("no debía leer".into()));
        let mut invalido = comando(empresa);
        invalido.cedula = "AB-123".into();
        assert_eq!(
            registrar(&almacen).ejecutar(&sesion(), &invalido).await,
            Err(ErrorCaso::Negocio(ErrorContratista::CedulaInvalida)),
            "falla por la regla, no por la lectura"
        );
    }

    #[tokio::test]
    async fn la_empresa_debe_existir() {
        let almacen = AlmacenMemoria::new();
        let sin_empresa = EmpresaId::desde_uuid(Uuid::from_u128(77));
        assert_eq!(
            registrar(&almacen)
                .ejecutar(&sesion(), &comando(sin_empresa))
                .await,
            Err(ErrorCaso::Negocio(ErrorContratista::EmpresaNoExiste))
        );
        assert!(almacen.contratistas().is_empty(), "no se guardó nada");
        assert!(almacen.auditoria().is_empty(), "ni se auditó");
    }

    #[tokio::test]
    async fn la_cedula_repetida_se_detecta_aunque_se_escriba_distinto() {
        let (almacen, empresa) = almacen_con_empresa();
        con_contratista(&almacen, empresa).await;
        let mut misma_persona = comando(empresa);
        misma_persona.cedula = "01-1111-1111".into();
        assert_eq!(
            registrar(&almacen)
                .ejecutar(&sesion(), &misma_persona)
                .await,
            Err(ErrorCaso::Negocio(ErrorContratista::CedulaRepetida))
        );
        assert_eq!(almacen.contratistas().len(), 1, "sigue habiendo uno solo");
    }

    #[tokio::test]
    async fn no_se_registra_con_el_praind_vencido() {
        let (almacen, empresa) = almacen_con_empresa();
        let mut vencido = comando(empresa);
        vencido.fecha_vencimiento_praind = fecha("2026-10-08");
        assert_eq!(
            registrar(&almacen).ejecutar(&sesion(), &vencido).await,
            Err(ErrorCaso::Negocio(ErrorContratista::PraindVencido))
        );
        assert!(almacen.contratistas().is_empty(), "no se guardó nada");
    }

    #[tokio::test]
    async fn el_choque_con_otro_equipo_al_confirmar_se_ve_como_cedula_repetida() {
        let (almacen, empresa) = almacen_con_empresa();
        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(
            Restriccion::CedulaContratista,
        ));
        assert_eq!(
            registrar(&almacen)
                .ejecutar(&sesion(), &comando(empresa))
                .await,
            Err(ErrorCaso::Negocio(ErrorContratista::CedulaRepetida)),
            "el operador ve lo mismo que si lo detectara la regla"
        );
    }

    #[tokio::test]
    async fn una_falla_tecnica_no_se_muestra_al_operador() {
        let (almacen, empresa) = almacen_con_empresa();
        almacen.fallar_proxima_confirmacion(ErrorPersistencia::Tecnica("disco lleno".into()));
        let error = registrar(&almacen)
            .ejecutar(&sesion(), &comando(empresa))
            .await
            .unwrap_err();
        let interfaz = error.para_interfaz();
        assert_eq!(interfaz.tipo, TipoError::Tecnico);
        assert_eq!(interfaz.mensaje, MENSAJE_ERROR_TECNICO);
        assert_eq!(interfaz.detalle_tecnico.as_deref(), Some("disco lleno"));
        assert!(almacen.contratistas().is_empty(), "no se guardó nada");
    }

    #[tokio::test]
    async fn una_falla_al_leer_es_tecnica() {
        let (almacen, empresa) = almacen_con_empresa();
        almacen.fallar_lecturas(ErrorPersistencia::Tecnica("caída".into()));
        let resultado = registrar(&almacen)
            .ejecutar(&sesion(), &comando(empresa))
            .await;
        assert!(
            matches!(resultado, Err(ErrorCaso::Tecnico(_))),
            "{resultado:?}"
        );
    }

    // --- Editar contratista ---

    #[tokio::test]
    async fn editar_uno_que_no_existe_es_no_encontrado() {
        let (almacen, empresa) = almacen_con_empresa();
        let inexistente = ContratistaId::desde_uuid(Uuid::from_u128(404));
        assert_eq!(
            editar(&almacen)
                .ejecutar(&sesion(), inexistente, &comando(empresa))
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn editar_guarda_y_audita_solo_lo_que_cambio() {
        let (almacen, empresa) = almacen_con_empresa();
        let id = con_contratista(&almacen, empresa).await;
        let mut cambio = comando(empresa);
        cambio.nombre = "jose pena".into();
        cambio.tiene_acceso = false;

        let cambios = editar(&almacen)
            .ejecutar(&sesion(), id, &cambio)
            .await
            .unwrap();
        let campos: Vec<_> = cambios.iter().map(|c| c.campo).collect();
        assert_eq!(campos, ["nombre", "tiene_acceso"]);

        let guardado = &almacen.contratistas()[0];
        assert_eq!(guardado.nombre().as_str(), "JOSE PENA");
        assert!(!guardado.tiene_acceso(), "se aplicó la edición");

        let auditoria = almacen.auditoria();
        assert_eq!(auditoria.len(), 2, "alta + edición");
        assert_eq!(auditoria[1].accion, AccionAuditada::Edicion);
        assert_eq!(auditoria[1].cambios, cambios, "lo mismo que se devolvió");
    }

    #[tokio::test]
    async fn editar_sin_cambios_no_escribe_nada() {
        let (almacen, empresa) = almacen_con_empresa();
        let id = con_contratista(&almacen, empresa).await;
        let cambios = editar(&almacen)
            .ejecutar(&sesion(), id, &comando(empresa))
            .await
            .unwrap();
        assert!(cambios.is_empty(), "nada cambió");
        assert_eq!(almacen.confirmaciones(), 1, "sólo la del alta");
        assert_eq!(almacen.auditoria().len(), 1, "sin auditoría vacía");
    }

    #[tokio::test]
    async fn la_misma_cedula_escrita_distinto_no_es_un_cambio() {
        let (almacen, empresa) = almacen_con_empresa();
        let id = con_contratista(&almacen, empresa).await;
        let mut otro_formato = comando(empresa);
        otro_formato.cedula = "111111111".into();
        let cambios = editar(&almacen)
            .ejecutar(&sesion(), id, &otro_formato)
            .await
            .unwrap();
        assert!(cambios.is_empty(), "la forma única es la misma");
    }

    #[tokio::test]
    async fn no_cambia_la_cedula_de_quien_esta_adentro() {
        let (almacen, empresa) = almacen_con_empresa();
        let id = con_contratista(&almacen, empresa).await;
        almacen.marcar_adentro(id);
        let mut otra_cedula = comando(empresa);
        otra_cedula.cedula = "222222222".into();
        assert_eq!(
            editar(&almacen).ejecutar(&sesion(), id, &otra_cedula).await,
            Err(ErrorCaso::Negocio(
                ErrorContratista::CedulaNoEditableAdentro
            ))
        );
        assert_eq!(almacen.contratistas()[0].cedula().as_str(), "111111111");
    }

    #[tokio::test]
    async fn no_puede_tomar_la_cedula_de_otro_contratista() {
        let (almacen, empresa) = almacen_con_empresa();
        let id = con_contratista(&almacen, empresa).await;
        let mut otro = comando(empresa);
        otro.cedula = "222222222".into();
        registrar(&almacen)
            .ejecutar(&sesion(), &otro)
            .await
            .unwrap();

        assert_eq!(
            editar(&almacen).ejecutar(&sesion(), id, &otro).await,
            Err(ErrorCaso::Negocio(ErrorContratista::CedulaRepetida))
        );
    }

    #[tokio::test]
    async fn con_el_praind_vencido_se_le_puede_quitar_el_acceso() {
        let (almacen, empresa) = almacen_con_empresa();
        let id = con_contratista(&almacen, empresa).await;
        let meses_despues = EditarContratista::new(almacen.clone(), reloj_en("2027-06-01"));
        let mut quitar_acceso = comando(empresa);
        quitar_acceso.tiene_acceso = false;
        assert!(
            meses_despues
                .ejecutar(&sesion(), id, &quitar_acceso)
                .await
                .is_ok(),
            "la fecha no cambió: no se revisa el vencimiento"
        );
    }

    // --- Consultar contratista ---

    #[tokio::test]
    async fn consulta_por_cedula_en_cualquier_formato_y_dice_si_puede_entrar() {
        let (almacen, empresa) = almacen_con_empresa();
        let mut por_vencer = comando(empresa);
        por_vencer.fecha_vencimiento_praind = fecha("2026-11-01");
        registrar(&almacen)
            .ejecutar(&sesion(), &por_vencer)
            .await
            .unwrap();

        let ficha = ConsultarContratista::new(almacen.clone(), reloj())
            .ejecutar("01-1111-1111")
            .await
            .unwrap();
        assert_eq!(ficha.contratista.nombre().as_str(), "JOSE PEÑA");
        assert_eq!(
            ficha.acceso,
            ResultadoAcceso::PermitidoConAdvertencia {
                dias_para_vencer: 23
            }
        );
    }

    #[tokio::test]
    async fn la_consulta_informa_el_acceso_denegado() {
        let (almacen, empresa) = almacen_con_empresa();
        let mut sin_acceso = comando(empresa);
        sin_acceso.tiene_acceso = false;
        registrar(&almacen)
            .ejecutar(&sesion(), &sin_acceso)
            .await
            .unwrap();

        let ficha = ConsultarContratista::new(almacen.clone(), reloj())
            .ejecutar("111111111")
            .await
            .unwrap();
        assert_eq!(
            ficha.acceso,
            ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso)
        );
    }

    #[tokio::test]
    async fn consultar_una_cedula_sin_registro_es_no_encontrado() {
        let almacen = AlmacenMemoria::new();
        assert_eq!(
            ConsultarContratista::new(almacen, reloj())
                .ejecutar("999999999")
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn consultar_una_cedula_invalida_es_error_de_negocio() {
        let almacen = AlmacenMemoria::new();
        assert_eq!(
            ConsultarContratista::new(almacen, reloj())
                .ejecutar("12")
                .await,
            Err(ErrorCaso::Negocio(ErrorContratista::CedulaInvalida))
        );
    }

    // --- Empresas ---

    fn registrar_empresa(
        almacen: &AlmacenMemoria,
    ) -> RegistrarEmpresa<AlmacenMemoria, RelojFijo, IdsSecuenciales> {
        RegistrarEmpresa::new(almacen.clone(), reloj(), almacen.ids())
    }

    fn renombrar_empresa(almacen: &AlmacenMemoria) -> RenombrarEmpresa<AlmacenMemoria, RelojFijo> {
        RenombrarEmpresa::new(almacen.clone(), reloj())
    }

    #[tokio::test]
    async fn registra_la_empresa_normalizada_y_audita_el_alta() {
        let almacen = AlmacenMemoria::new();
        let id = registrar_empresa(&almacen)
            .ejecutar(&sesion(), "  acme  s.a. ")
            .await
            .unwrap();
        assert_eq!(almacen.empresas()[0].nombre().as_str(), "ACME S.A.");
        let auditoria = almacen.auditoria();
        assert_eq!(auditoria.len(), 1);
        assert_eq!(auditoria[0].entidad, EntidadAuditada::Empresa);
        assert_eq!(auditoria[0].id, id.uuid());
    }

    #[tokio::test]
    async fn el_nombre_de_empresa_no_se_repite_aunque_se_escriba_distinto() {
        let (almacen, _) = almacen_con_empresa();
        assert_eq!(
            registrar_empresa(&almacen)
                .ejecutar(&sesion(), " acme ")
                .await,
            Err(ErrorCaso::Negocio(ErrorEmpresa::NombreRepetido))
        );
    }

    #[tokio::test]
    async fn un_nombre_vacio_se_rechaza_sin_tocar_la_base() {
        let almacen = AlmacenMemoria::new();
        almacen.fallar_lecturas(ErrorPersistencia::Tecnica("no debía leer".into()));
        assert_eq!(
            registrar_empresa(&almacen).ejecutar(&sesion(), "   ").await,
            Err(ErrorCaso::Negocio(ErrorEmpresa::NombreVacio))
        );
    }

    #[tokio::test]
    async fn el_choque_de_nombre_al_confirmar_se_ve_como_nombre_repetido() {
        let almacen = AlmacenMemoria::new();
        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::NombreEmpresa));
        assert_eq!(
            registrar_empresa(&almacen)
                .ejecutar(&sesion(), "ACME")
                .await,
            Err(ErrorCaso::Negocio(ErrorEmpresa::NombreRepetido))
        );
    }

    #[tokio::test]
    async fn renombrar_una_empresa_que_no_existe_es_no_encontrado() {
        let almacen = AlmacenMemoria::new();
        let inexistente = EmpresaId::desde_uuid(Uuid::from_u128(404));
        assert_eq!(
            renombrar_empresa(&almacen)
                .ejecutar(&sesion(), inexistente, "OTRA")
                .await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn renombrar_guarda_y_audita_el_cambio() {
        let (almacen, id) = almacen_con_empresa();
        let cambios = renombrar_empresa(&almacen)
            .ejecutar(&sesion(), id, "acme costa rica")
            .await
            .unwrap();
        assert_eq!(cambios.len(), 1);
        assert_eq!(cambios[0].antes, "ACME");
        assert_eq!(cambios[0].despues, "ACME COSTA RICA");
        assert_eq!(almacen.empresas()[0].nombre().as_str(), "ACME COSTA RICA");
        assert_eq!(almacen.auditoria()[0].accion, AccionAuditada::Edicion);
    }

    #[tokio::test]
    async fn renombrar_con_el_mismo_nombre_no_escribe_nada() {
        let (almacen, id) = almacen_con_empresa();
        let cambios = renombrar_empresa(&almacen)
            .ejecutar(&sesion(), id, " acme ")
            .await
            .unwrap();
        assert!(cambios.is_empty(), "es el mismo nombre");
        assert_eq!(almacen.confirmaciones(), 0, "no se confirmó nada");
        assert!(almacen.auditoria().is_empty(), "ni se auditó");
    }

    #[tokio::test]
    async fn no_se_puede_renombrar_con_el_nombre_de_otra_empresa() {
        let (almacen, id) = almacen_con_empresa();
        registrar_empresa(&almacen)
            .ejecutar(&sesion(), "FEMSA")
            .await
            .unwrap();
        assert_eq!(
            renombrar_empresa(&almacen)
                .ejecutar(&sesion(), id, "femsa")
                .await,
            Err(ErrorCaso::Negocio(ErrorEmpresa::NombreRepetido))
        );
    }
}
