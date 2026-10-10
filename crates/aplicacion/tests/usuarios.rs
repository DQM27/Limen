//! Pruebas de los casos de uso de usuarios e inicio de sesión contra los
//! dobles en memoria.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
    use limen_aplicacion::casos_de_uso::usuarios::{
        CambiarContrasena, CrearPrimerUsuario, EditarUsuario, HayUsuarios, IniciarSesion,
        ListarUsuarios, RegistrarUsuario, RestablecerContrasena, SesionIniciada,
    };
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::{
        AccionAuditada, ErrorPersistencia, RegistroAuditado, Restriccion,
    };
    use limen_aplicacion::sesion::OperadorId;
    use limen_dominio::cedula::Cedula;
    use limen_dominio::usuario::{ErrorInicioSesion, ErrorUsuario};
    use limen_infra_memoria::{AlmacenMemoria, ContrasenasFalsas, IdsSecuenciales, RelojFijo};

    const AHORA: &str = "2026-10-10T08:00:00Z";
    const CEDULA_ANA: &str = "1-1111-1111";
    const CLAVE_ANA: &str = "portería segura";

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    fn reloj_en(ahora: DateTime<Utc>) -> RelojFijo {
        RelojFijo::new(ahora, NaiveDate::from_ymd_opt(2026, 10, 10).unwrap())
    }

    fn reloj() -> RelojFijo {
        reloj_en(instante(AHORA))
    }

    fn primer_usuario(
        almacen: &AlmacenMemoria,
    ) -> CrearPrimerUsuario<AlmacenMemoria, RelojFijo, IdsSecuenciales, ContrasenasFalsas> {
        CrearPrimerUsuario::new(almacen.clone(), reloj(), almacen.ids(), ContrasenasFalsas)
    }

    fn registrar(
        almacen: &AlmacenMemoria,
    ) -> RegistrarUsuario<AlmacenMemoria, RelojFijo, IdsSecuenciales, ContrasenasFalsas> {
        RegistrarUsuario::new(almacen.clone(), reloj(), almacen.ids(), ContrasenasFalsas)
    }

    fn iniciar_en(
        almacen: &AlmacenMemoria,
        ahora: DateTime<Utc>,
    ) -> IniciarSesion<AlmacenMemoria, RelojFijo, ContrasenasFalsas> {
        IniciarSesion::new(almacen.clone(), reloj_en(ahora), ContrasenasFalsas)
    }

    /// Un equipo con Ana como primer usuario, ya con su sesión.
    async fn con_ana() -> (AlmacenMemoria, SesionIniciada) {
        let almacen = AlmacenMemoria::new();
        let ana = primer_usuario(&almacen)
            .ejecutar(CEDULA_ANA, "ana mora", CLAVE_ANA)
            .await
            .unwrap();
        (almacen, ana)
    }

    fn cedula(texto: &str) -> Cedula {
        Cedula::normalizar(texto).unwrap()
    }

    #[tokio::test]
    async fn el_primer_usuario_abre_su_sesion_y_luego_ya_no_se_puede() {
        let almacen = AlmacenMemoria::new();
        let hay = HayUsuarios::new(almacen.clone());
        assert!(!hay.ejecutar().await.unwrap(), "equipo recién instalado");

        let ana = primer_usuario(&almacen)
            .ejecutar(CEDULA_ANA, "ana mora", CLAVE_ANA)
            .await
            .unwrap();
        assert_eq!(ana.cedula, "111111111");
        assert_eq!(ana.nombre, "ANA MORA");
        assert!(!ana.debe_cambiar_contrasena, "eligió su contraseña");
        assert!(hay.ejecutar().await.unwrap(), "ya hay usuarios");

        let auditoria = almacen.auditoria();
        let [alta] = auditoria.as_slice() else {
            panic!("se audita el alta: {auditoria:?}");
        };
        assert_eq!(
            alta.registro,
            RegistroAuditado::Usuario(ana.sesion.operador())
        );
        assert_eq!(
            alta.operador,
            ana.sesion.operador(),
            "se dio de alta a sí mismo"
        );
        assert!(
            alta.cambios
                .iter()
                .all(|cambio| cambio.campo != "contrasena"),
            "la contraseña no va a la auditoría: {:?}",
            alta.cambios
        );

        let segundo = primer_usuario(&almacen)
            .ejecutar("222222222", "beto solís", "otra clave larga")
            .await;
        assert_eq!(
            segundo,
            Err(ErrorCaso::Negocio(ErrorUsuario::YaHayUsuarios))
        );
    }

    #[tokio::test]
    async fn la_contrasena_se_guarda_cifrada() {
        let (almacen, _) = con_ana().await;
        let usuarios = almacen.usuarios();
        let [ana] = usuarios.as_slice() else {
            panic!("un usuario: {usuarios:?}");
        };
        assert_eq!(ana.contrasena(), &ContrasenasFalsas::hash_de(CLAVE_ANA));
    }

    #[tokio::test]
    async fn inicia_sesion_con_cedula_y_contrasena() {
        let (almacen, ana) = con_ana().await;
        let iniciada = iniciar_en(&almacen, instante(AHORA))
            .ejecutar("111111111", CLAVE_ANA)
            .await
            .unwrap();
        assert_eq!(iniciada, ana);
    }

    #[tokio::test]
    async fn cedula_desconocida_y_contrasena_equivocada_dan_el_mismo_error() {
        let (almacen, _) = con_ana().await;
        let iniciar = iniciar_en(&almacen, instante(AHORA));
        let esperado = Err(ErrorCaso::Negocio(ErrorInicioSesion::CredencialesInvalidas));
        assert_eq!(
            iniciar.ejecutar(CEDULA_ANA, "no es la clave").await,
            esperado
        );
        assert_eq!(iniciar.ejecutar("999999999", CLAVE_ANA).await, esperado);
        assert_eq!(iniciar.ejecutar("abc", CLAVE_ANA).await, esperado);
    }

    #[tokio::test]
    async fn cinco_fallos_bloquean_aunque_despues_acierte() {
        let (almacen, _) = con_ana().await;
        let ahora = instante(AHORA);
        let iniciar = iniciar_en(&almacen, ahora);
        for _ in 0..5 {
            let fallo = iniciar.ejecutar(CEDULA_ANA, "equivocada").await;
            assert_eq!(
                fallo,
                Err(ErrorCaso::Negocio(ErrorInicioSesion::CredencialesInvalidas))
            );
        }
        assert_eq!(
            iniciar.ejecutar(CEDULA_ANA, CLAVE_ANA).await,
            Err(ErrorCaso::Negocio(ErrorInicioSesion::Bloqueado {
                minutos: 5
            })),
            "bloqueado aunque la contraseña sea la correcta"
        );

        let despues = iniciar_en(&almacen, ahora + TimeDelta::minutes(5));
        assert!(
            despues.ejecutar(CEDULA_ANA, CLAVE_ANA).await.is_ok(),
            "pasado el bloqueo vuelve a entrar"
        );
        assert_eq!(
            almacen.intentos_inicio(&cedula(CEDULA_ANA)),
            None,
            "entrar borra los intentos fallidos"
        );
    }

    #[tokio::test]
    async fn tambien_se_bloquea_una_cedula_que_no_existe() {
        let almacen = AlmacenMemoria::new();
        let iniciar = iniciar_en(&almacen, instante(AHORA));
        for _ in 0..5 {
            iniciar
                .ejecutar("999999999", "lo que sea")
                .await
                .unwrap_err();
        }
        assert_eq!(
            iniciar.ejecutar("999999999", "lo que sea").await,
            Err(ErrorCaso::Negocio(ErrorInicioSesion::Bloqueado {
                minutos: 5
            })),
            "si no se bloqueara, se notaría que la cédula no existe"
        );
    }

    #[tokio::test]
    async fn al_desactivado_se_le_avisa_solo_con_la_contrasena_correcta() {
        let (almacen, ana) = con_ana().await;
        let beto = registrar(&almacen)
            .ejecutar(&ana.sesion, "222222222", "beto solís", "temporal 123")
            .await
            .unwrap();
        EditarUsuario::new(almacen.clone(), reloj(), almacen.ids())
            .ejecutar(&ana.sesion, beto, "beto solís", false)
            .await
            .unwrap();

        let iniciar = iniciar_en(&almacen, instante(AHORA));
        assert_eq!(
            iniciar.ejecutar("222222222", "equivocada").await,
            Err(ErrorCaso::Negocio(ErrorInicioSesion::CredencialesInvalidas))
        );
        assert_eq!(
            iniciar.ejecutar("222222222", "temporal 123").await,
            Err(ErrorCaso::Negocio(ErrorInicioSesion::Desactivado))
        );
        assert_eq!(
            almacen
                .intentos_inicio(&cedula("222222222"))
                .map(|intentos| intentos.cantidad),
            Some(1),
            "el desactivado con la contraseña correcta no suma intento"
        );
    }

    #[tokio::test]
    async fn el_registrado_entra_con_una_contrasena_temporal() {
        let (almacen, ana) = con_ana().await;
        registrar(&almacen)
            .ejecutar(&ana.sesion, "222222222", "beto solís", "temporal 123")
            .await
            .unwrap();
        let beto = iniciar_en(&almacen, instante(AHORA))
            .ejecutar("222222222", "temporal 123")
            .await
            .unwrap();
        assert!(beto.debe_cambiar_contrasena, "la eligió otra persona");

        CambiarContrasena::new(almacen.clone(), reloj(), almacen.ids(), ContrasenasFalsas)
            .ejecutar(&beto.sesion, "temporal 123", "la mía de verdad")
            .await
            .unwrap();
        let beto = iniciar_en(&almacen, instante(AHORA))
            .ejecutar("222222222", "la mía de verdad")
            .await
            .unwrap();
        assert!(!beto.debe_cambiar_contrasena, "ya la cambió");
    }

    #[tokio::test]
    async fn la_cedula_de_usuario_no_se_repite_ni_aunque_otro_equipo_gane() {
        let (almacen, ana) = con_ana().await;
        let repetida = registrar(&almacen)
            .ejecutar(&ana.sesion, CEDULA_ANA, "otra ana", "temporal 123")
            .await;
        assert_eq!(
            repetida,
            Err(ErrorCaso::Negocio(ErrorUsuario::CedulaRepetida))
        );

        almacen
            .fallar_proxima_confirmacion(ErrorPersistencia::Conflicto(Restriccion::CedulaUsuario));
        let carrera = registrar(&almacen)
            .ejecutar(&ana.sesion, "222222222", "beto solís", "temporal 123")
            .await;
        assert_eq!(
            carrera,
            Err(ErrorCaso::Negocio(ErrorUsuario::CedulaRepetida))
        );
    }

    #[tokio::test]
    async fn la_contrasena_nueva_cumple_l3() {
        let (almacen, ana) = con_ana().await;
        let registrar = registrar(&almacen);
        assert_eq!(
            registrar
                .ejecutar(&ana.sesion, "222222222", "beto solís", "corta")
                .await,
            Err(ErrorCaso::Negocio(ErrorUsuario::ContrasenaCorta))
        );
        assert_eq!(
            registrar
                .ejecutar(&ana.sesion, "222222222", "beto solís", "222222222")
                .await,
            Err(ErrorCaso::Negocio(ErrorUsuario::ContrasenaIgualALaCedula))
        );
        assert_eq!(almacen.usuarios().len(), 1, "no se guardó nada");
    }

    #[tokio::test]
    async fn cambiar_la_contrasena_pide_la_actual() {
        let (almacen, ana) = con_ana().await;
        let cambio =
            CambiarContrasena::new(almacen.clone(), reloj(), almacen.ids(), ContrasenasFalsas)
                .ejecutar(&ana.sesion, "no es la actual", "otra clave larga")
                .await;
        assert_eq!(
            cambio,
            Err(ErrorCaso::Negocio(ErrorUsuario::ContrasenaActualIncorrecta))
        );
        assert!(
            iniciar_en(&almacen, instante(AHORA))
                .ejecutar(CEDULA_ANA, CLAVE_ANA)
                .await
                .is_ok(),
            "la contraseña sigue igual"
        );
    }

    #[tokio::test]
    async fn restablecer_deja_una_temporal_y_se_audita_sin_el_hash() {
        let (almacen, ana) = con_ana().await;
        let beto = registrar(&almacen)
            .ejecutar(&ana.sesion, "222222222", "beto solís", "temporal 123")
            .await
            .unwrap();
        RestablecerContrasena::new(almacen.clone(), reloj(), almacen.ids(), ContrasenasFalsas)
            .ejecutar(&ana.sesion, beto, "nueva temporal")
            .await
            .unwrap();
        let iniciada = iniciar_en(&almacen, instante(AHORA))
            .ejecutar("222222222", "nueva temporal")
            .await
            .unwrap();
        assert!(iniciada.debe_cambiar_contrasena, "queda temporal");

        let auditoria = almacen.auditoria();
        let Some(ultima) = auditoria.last() else {
            panic!("hay auditoría");
        };
        assert_eq!(ultima.accion, AccionAuditada::Edicion);
        assert_eq!(ultima.operador, ana.sesion.operador(), "quien restableció");
        assert!(
            ultima
                .cambios
                .iter()
                .all(|cambio| !cambio.despues.contains("falso:")),
            "el hash no va a la auditoría: {:?}",
            ultima.cambios
        );
    }

    #[tokio::test]
    async fn nadie_se_desactiva_a_si_mismo_y_editar_lo_mismo_no_escribe() {
        let (almacen, ana) = con_ana().await;
        let editar = EditarUsuario::new(almacen.clone(), reloj(), almacen.ids());
        assert_eq!(
            editar
                .ejecutar(&ana.sesion, ana.sesion.operador(), "ana mora", false)
                .await,
            Err(ErrorCaso::Negocio(ErrorUsuario::NoSeDesactivaASiMismo))
        );
        let confirmaciones = almacen.confirmaciones();
        let cambios = editar
            .ejecutar(&ana.sesion, ana.sesion.operador(), "ana mora", true)
            .await
            .unwrap();
        assert!(cambios.is_empty(), "nada cambió: {cambios:?}");
        assert_eq!(almacen.confirmaciones(), confirmaciones, "no se escribió");

        let otro = OperadorId::desde_uuid(uuid::Uuid::from_u128(77));
        assert_eq!(
            editar.ejecutar(&ana.sesion, otro, "nadie", true).await,
            Err(ErrorCaso::NoEncontrado)
        );
    }

    #[tokio::test]
    async fn la_lista_va_por_nombre_y_sin_contrasenas() {
        let (almacen, ana) = con_ana().await;
        registrar(&almacen)
            .ejecutar(&ana.sesion, "222222222", "abel rojas", "temporal 123")
            .await
            .unwrap();
        let lista = ListarUsuarios::new(almacen.clone())
            .ejecutar()
            .await
            .unwrap();
        let nombres: Vec<&str> = lista.iter().map(|fila| fila.nombre.as_str()).collect();
        assert_eq!(nombres, ["ABEL ROJAS", "ANA MORA"]);
        assert!(
            lista
                .iter()
                .all(|fila| fila.activo && fila.cedula.len() == 9),
            "{lista:?}"
        );
    }
}
