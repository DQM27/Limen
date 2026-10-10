//! Pruebas de los casos de uso de la sesión (iniciar sesión y cambiar la
//! propia clave) contra los dobles en memoria. Los usuarios no se crean en el
//! equipo (vienen de la nube): las pruebas los siembran en la base.

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
    use limen_aplicacion::casos_de_uso::usuarios::{CambiarClave, IniciarSesion, SesionIniciada};
    use limen_aplicacion::errores::ErrorCaso;
    use limen_aplicacion::puertos::{AccionAuditada, RegistroAuditado};
    use limen_aplicacion::sesion::{OperadorId, Sesion};
    use limen_dominio::cedula::Cedula;
    use limen_dominio::nombre::NombrePersona;
    use limen_dominio::usuario::{ErrorInicioSesion, ErrorUsuario, Usuario, UsuarioGuardado};
    use limen_infra_memoria::{AlmacenMemoria, ClavesFalsas, IdsSecuenciales, RelojFijo};
    use uuid::Uuid;

    const AHORA: &str = "2026-10-10T08:00:00Z";
    const CEDULA_ANA: &str = "1-1111-1111";
    const CLAVE_ANA: &str = "portería segura";
    const ANA: OperadorId = OperadorId::desde_uuid(Uuid::from_u128(1));
    const BETO: OperadorId = OperadorId::desde_uuid(Uuid::from_u128(2));

    fn instante(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    fn reloj_en(ahora: DateTime<Utc>) -> RelojFijo {
        RelojFijo::new(ahora, NaiveDate::from_ymd_opt(2026, 10, 10).unwrap())
    }

    fn reloj() -> RelojFijo {
        reloj_en(instante(AHORA))
    }

    fn iniciar_en(
        almacen: &AlmacenMemoria,
        ahora: DateTime<Utc>,
    ) -> IniciarSesion<AlmacenMemoria, RelojFijo, ClavesFalsas> {
        IniciarSesion::new(almacen.clone(), reloj_en(ahora), ClavesFalsas)
    }

    fn cambiar(
        almacen: &AlmacenMemoria,
    ) -> CambiarClave<AlmacenMemoria, RelojFijo, IdsSecuenciales, ClavesFalsas> {
        CambiarClave::new(almacen.clone(), reloj(), almacen.ids(), ClavesFalsas)
    }

    fn cedula(texto: &str) -> Cedula {
        Cedula::normalizar(texto).unwrap()
    }

    /// Un usuario como llega de la nube.
    fn usuario(
        id: OperadorId,
        cedula_texto: &str,
        nombre: &str,
        clave: &str,
        activo: bool,
        temporal: bool,
    ) -> Usuario {
        Usuario::restaurar(UsuarioGuardado {
            id,
            cedula: cedula(cedula_texto),
            nombre: NombrePersona::nuevo(nombre).unwrap(),
            activo,
            clave: ClavesFalsas::hash_de(clave),
            debe_cambiar_clave: temporal,
        })
    }

    /// La base de un equipo con los usuarios que trajo la nube: Ana (clave
    /// propia), Beto (clave temporal) y Carla (desactivada).
    fn equipo() -> AlmacenMemoria {
        let almacen = AlmacenMemoria::new();
        almacen.sembrar_usuario(usuario(ANA, CEDULA_ANA, "ana mora", CLAVE_ANA, true, false));
        almacen.sembrar_usuario(usuario(
            BETO,
            "222222222",
            "beto solís",
            "temporal 123",
            true,
            true,
        ));
        almacen.sembrar_usuario(usuario(
            OperadorId::desde_uuid(Uuid::from_u128(3)),
            "333333333",
            "carla vega",
            "la de carla",
            false,
            false,
        ));
        almacen
    }

    #[tokio::test]
    async fn inicia_sesion_con_cedula_y_clave() {
        let almacen = equipo();
        let iniciada = iniciar_en(&almacen, instante(AHORA))
            .ejecutar("111111111", CLAVE_ANA)
            .await
            .unwrap();
        assert_eq!(
            iniciada,
            SesionIniciada {
                sesion: Sesion::nueva(ANA),
                cedula: "111111111".to_owned(),
                nombre: "ANA MORA".to_owned(),
                debe_cambiar_clave: false,
            }
        );
    }

    #[tokio::test]
    async fn cedula_desconocida_y_clave_equivocada_dan_el_mismo_error() {
        let almacen = equipo();
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
        let almacen = equipo();
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
            "bloqueado aunque la clave sea la correcta"
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
    async fn cambiar_la_clave_pide_la_actual() {
        let almacen = equipo();
        let cambio = cambiar(&almacen)
            .ejecutar(&Sesion::nueva(ANA), "no es la actual", "otra clave larga")
            .await;
        assert_eq!(
            cambio,
            Err(ErrorCaso::Negocio(ErrorUsuario::ClaveActualIncorrecta))
        );
        assert!(
            iniciar_en(&almacen, instante(AHORA))
                .ejecutar(CEDULA_ANA, CLAVE_ANA)
                .await
                .is_ok(),
            "la clave sigue igual"
        );
    }

    #[tokio::test]
    async fn al_desactivado_se_le_avisa_solo_con_la_clave_correcta() {
        let almacen = equipo();
        let iniciar = iniciar_en(&almacen, instante(AHORA));
        assert_eq!(
            iniciar.ejecutar("333333333", "equivocada").await,
            Err(ErrorCaso::Negocio(ErrorInicioSesion::CredencialesInvalidas))
        );
        assert_eq!(
            iniciar.ejecutar("333333333", "la de carla").await,
            Err(ErrorCaso::Negocio(ErrorInicioSesion::Desactivado))
        );
        assert_eq!(
            almacen
                .intentos_inicio(&cedula("333333333"))
                .map(|intentos| intentos.cantidad),
            Some(1),
            "la desactivada con la clave correcta no suma intento"
        );
    }

    #[tokio::test]
    async fn con_clave_temporal_entra_y_la_cambia() {
        let almacen = equipo();
        let beto = iniciar_en(&almacen, instante(AHORA))
            .ejecutar("222222222", "temporal 123")
            .await
            .unwrap();
        assert!(beto.debe_cambiar_clave, "la puso la nube");

        cambiar(&almacen)
            .ejecutar(&beto.sesion, "temporal 123", "la mía de verdad")
            .await
            .unwrap();
        let beto = iniciar_en(&almacen, instante(AHORA))
            .ejecutar("222222222", "la mía de verdad")
            .await
            .unwrap();
        assert!(!beto.debe_cambiar_clave, "ya la cambió");

        let auditoria = almacen.auditoria();
        let [cambio] = auditoria.as_slice() else {
            panic!("se audita el cambio: {auditoria:?}");
        };
        assert_eq!(cambio.registro, RegistroAuditado::Usuario(BETO));
        assert_eq!(cambio.accion, AccionAuditada::Edicion);
        assert_eq!(cambio.operador, BETO, "la cambió él mismo");
        assert!(
            cambio
                .cambios
                .iter()
                .all(|campo| !campo.despues.contains("falso:")),
            "el hash no va a la auditoría: {:?}",
            cambio.cambios
        );
    }

    #[tokio::test]
    async fn la_clave_nueva_cumple_l3_y_se_guarda_cifrada() {
        let almacen = equipo();
        let sesion = Sesion::nueva(ANA);
        assert_eq!(
            cambiar(&almacen)
                .ejecutar(&sesion, CLAVE_ANA, "corta")
                .await,
            Err(ErrorCaso::Negocio(ErrorUsuario::ClaveCorta))
        );
        assert_eq!(
            cambiar(&almacen)
                .ejecutar(&sesion, CLAVE_ANA, "111111111")
                .await,
            Err(ErrorCaso::Negocio(ErrorUsuario::ClaveIgualALaCedula))
        );
        cambiar(&almacen)
            .ejecutar(&sesion, CLAVE_ANA, "otra clave larga")
            .await
            .unwrap();
        let usuarios = almacen.usuarios();
        let Some(ana) = usuarios.iter().find(|usuario| usuario.id() == ANA) else {
            panic!("está Ana: {usuarios:?}");
        };
        assert_eq!(ana.clave(), &ClavesFalsas::hash_de("otra clave larga"));
    }
}
