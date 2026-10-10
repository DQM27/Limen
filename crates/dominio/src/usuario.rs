//! Usuarios: quienes operan el sistema (bloque L de `docs/reglas.md`).
//!
//! - L1: un solo rol, Operador. Su identificador es el [`OperadorId`] que
//!   queda en cada hecho y en cada entrada de la auditoría.
//! - L2: se desactivan, no se borran. Nadie se desactiva a sí mismo (así el
//!   equipo nunca se queda sin quién entre).
//! - L3: la clave tiene entre 8 y 128 caracteres y no puede ser la
//!   propia cédula. El máximo evita que un texto enorme trabe al cifrador.
//! - L6: tras 5 intentos fallidos seguidos con la misma cédula, se bloquea
//!   el inicio de sesión de esa cédula por 5 minutos.
//!
//! El inicio de sesión no revela si una cédula existe: cédula desconocida o
//! clave equivocada dan el mismo error, y "desactivado" sólo se dice a
//! quien escribió la clave correcta.
//!
//! El dominio no cifra ni verifica claves (eso necesita sal aleatoria y
//! es lento): recibe el hash ya hecho y el resultado de la verificación.

use std::fmt;

use chrono::{DateTime, TimeDelta, Utc};

use crate::auditoria::{CambioCampo, CamposAuditables, cambios_de_alta, diferencias};
use crate::cedula::Cedula;
use crate::nombre::{NombreInvalido, NombrePersona};
use crate::operador::OperadorId;

pub const LARGO_MINIMO_CLAVE: usize = 8;
pub const LARGO_MAXIMO_CLAVE: usize = 128;
/// Intentos fallidos seguidos que bloquean una cédula (L6).
pub const INTENTOS_ANTES_DEL_BLOQUEO: u32 = 5;
/// Cuánto dura el bloqueo, y también la ventana en que se cuentan los
/// intentos: un fallo más viejo que esto ya no suma.
pub const MINUTOS_DE_BLOQUEO: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorUsuario {
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    #[error("La cédula debe tener sólo números, entre 9 y 13 dígitos")]
    CedulaInvalida,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("El nombre sólo puede tener letras y espacios")]
    NombreInvalido,
    #[error("Ya existe un usuario con esa cédula")]
    CedulaRepetida,
    #[error("La clave debe tener al menos 8 caracteres")]
    ClaveCorta,
    #[error("La clave admite hasta 128 caracteres")]
    ClaveLarga,
    #[error("La clave no puede ser la cédula")]
    ClaveIgualALaCedula,
    #[error("La clave actual no es correcta")]
    ClaveActualIncorrecta,
    #[error("No puede desactivar su propio usuario")]
    NoSeDesactivaASiMismo,
    #[error("Ya hay usuarios: pídale a uno de ellos que lo registre")]
    YaHayUsuarios,
}

impl ErrorUsuario {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::CedulaVacia => "cedula_vacia",
            Self::CedulaInvalida => "cedula_invalida",
            Self::NombreVacio => "nombre_vacio",
            Self::NombreInvalido => "nombre_invalido",
            Self::CedulaRepetida => "usuario_cedula_repetida",
            Self::ClaveCorta => "clave_corta",
            Self::ClaveLarga => "clave_larga",
            Self::ClaveIgualALaCedula => "clave_igual_a_la_cedula",
            Self::ClaveActualIncorrecta => "clave_actual_incorrecta",
            Self::NoSeDesactivaASiMismo => "no_se_desactiva_a_si_mismo",
            Self::YaHayUsuarios => "ya_hay_usuarios",
        }
    }
}

/// Cédula de un usuario: nacional o de extranjero (A3), en su forma única.
pub fn cedula_de_usuario(texto: &str) -> Result<Cedula, ErrorUsuario> {
    if texto.trim().is_empty() {
        return Err(ErrorUsuario::CedulaVacia);
    }
    Cedula::normalizar(texto)
        .ok()
        .filter(Cedula::es_nacional_o_de_extranjero)
        .ok_or(ErrorUsuario::CedulaInvalida)
}

fn nombre_de(texto: &str) -> Result<NombrePersona, ErrorUsuario> {
    NombrePersona::nuevo(texto).map_err(|error| match error {
        NombreInvalido::Vacio => ErrorUsuario::NombreVacio,
        NombreInvalido::CaracteresNoPermitidos => ErrorUsuario::NombreInvalido,
    })
}

/// Una clave nueva que ya cumple L3, lista para cifrar. Nunca se
/// muestra ni se guarda: sólo su hash.
#[derive(Clone, PartialEq, Eq)]
pub struct ClaveNueva(String);

impl ClaveNueva {
    pub fn nueva(texto: &str, cedula: &Cedula) -> Result<Self, ErrorUsuario> {
        let largo = texto.chars().count();
        if largo < LARGO_MINIMO_CLAVE {
            return Err(ErrorUsuario::ClaveCorta);
        }
        if largo > LARGO_MAXIMO_CLAVE {
            return Err(ErrorUsuario::ClaveLarga);
        }
        let es_la_cedula = Cedula::normalizar(texto).is_ok_and(|escrita| &escrita == cedula);
        if es_la_cedula {
            return Err(ErrorUsuario::ClaveIgualALaCedula);
        }
        Ok(Self(texto.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ClaveNueva {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClaveNueva(«oculta»)")
    }
}

/// El hash de una clave, tal como lo produce el cifrador (formato
/// PHC). Para el dominio es opaco: sólo lo guarda.
#[derive(Clone, PartialEq, Eq)]
pub struct HashClave(String);

impl HashClave {
    pub const fn desde_texto(texto: String) -> Self {
        Self(texto)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for HashClave {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HashClave(«oculto»)")
    }
}

/// Lo que el caso de uso averiguó antes de dar de alta a un usuario.
#[derive(Debug, Clone, Copy)]
pub struct HechosUsuario {
    pub cedula_en_uso: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usuario {
    id: OperadorId,
    cedula: Cedula,
    nombre: NombrePersona,
    activo: bool,
    clave: HashClave,
    debe_cambiar_clave: bool,
}

/// Datos de un usuario guardado, para reconstruirlo.
#[derive(Debug, Clone)]
pub struct UsuarioGuardado {
    pub id: OperadorId,
    pub cedula: Cedula,
    pub nombre: NombrePersona,
    pub activo: bool,
    pub clave: HashClave,
    pub debe_cambiar_clave: bool,
}

impl Usuario {
    /// Da de alta a un usuario activo. `temporal`: la clave la eligió
    /// otra persona, así que debe cambiarla al entrar.
    pub fn registrar(
        id: OperadorId,
        cedula: Cedula,
        nombre: &str,
        clave: HashClave,
        temporal: bool,
        hechos: HechosUsuario,
    ) -> Result<Self, ErrorUsuario> {
        let nombre = nombre_de(nombre)?;
        if hechos.cedula_en_uso {
            return Err(ErrorUsuario::CedulaRepetida);
        }
        Ok(Self {
            id,
            cedula,
            nombre,
            activo: true,
            clave,
            debe_cambiar_clave: temporal,
        })
    }

    /// El primer usuario de un equipo recién instalado: lo crea quien
    /// instala, sin sesión, y sólo mientras no haya ningún otro. Su
    /// clave la eligió él mismo, así que no es temporal.
    pub fn crear_primero(
        id: OperadorId,
        cedula: Cedula,
        nombre: &str,
        clave: HashClave,
        hay_usuarios: bool,
    ) -> Result<Self, ErrorUsuario> {
        if hay_usuarios {
            return Err(ErrorUsuario::YaHayUsuarios);
        }
        Self::registrar(
            id,
            cedula,
            nombre,
            clave,
            false,
            HechosUsuario {
                cedula_en_uso: false,
            },
        )
    }

    /// Cambia el nombre y si está activo (L2). Nadie se desactiva a sí
    /// mismo. La cédula no se edita: es su identidad para entrar.
    pub fn editar(
        &mut self,
        nombre: &str,
        activo: bool,
        quien_edita: OperadorId,
    ) -> Result<Vec<CambioCampo>, ErrorUsuario> {
        let nombre = nombre_de(nombre)?;
        if !activo && quien_edita == self.id {
            return Err(ErrorUsuario::NoSeDesactivaASiMismo);
        }
        let antes = self.campos_auditables();
        self.nombre = nombre;
        self.activo = activo;
        Ok(diferencias(antes, self.campos_auditables()))
    }

    /// Cambia la propia clave: hay que saber la actual. Deja de ser
    /// temporal.
    pub fn cambiar_clave(
        &mut self,
        actual_correcta: bool,
        nueva: HashClave,
    ) -> Result<Vec<CambioCampo>, ErrorUsuario> {
        if !actual_correcta {
            return Err(ErrorUsuario::ClaveActualIncorrecta);
        }
        Ok(self.poner_clave(nueva, false))
    }

    /// Otra persona le pone una clave nueva: queda temporal y debe
    /// cambiarla al entrar.
    pub fn restablecer_clave(&mut self, nueva: HashClave) -> Vec<CambioCampo> {
        self.poner_clave(nueva, true)
    }

    fn poner_clave(&mut self, nueva: HashClave, temporal: bool) -> Vec<CambioCampo> {
        let antes = self.campos_auditables();
        self.clave = nueva;
        self.debe_cambiar_clave = temporal;
        // La clave nunca va a la auditoría: sólo que cambió.
        let mut cambios = vec![CambioCampo {
            campo: "clave",
            antes: String::new(),
            despues: "cambiada".to_owned(),
        }];
        cambios.extend(diferencias(antes, self.campos_auditables()));
        cambios
    }

    /// Lo que la auditoría registra del alta (nunca la clave).
    pub fn cambios_de_alta(&self) -> Vec<CambioCampo> {
        cambios_de_alta(self.campos_auditables())
    }

    fn campos_auditables(&self) -> CamposAuditables {
        vec![
            ("cedula", self.cedula.to_string()),
            ("nombre", self.nombre.to_string()),
            ("activo", self.activo.to_string()),
            ("debe_cambiar_clave", self.debe_cambiar_clave.to_string()),
        ]
    }

    pub fn restaurar(guardado: UsuarioGuardado) -> Self {
        Self {
            id: guardado.id,
            cedula: guardado.cedula,
            nombre: guardado.nombre,
            activo: guardado.activo,
            clave: guardado.clave,
            debe_cambiar_clave: guardado.debe_cambiar_clave,
        }
    }

    pub const fn id(&self) -> OperadorId {
        self.id
    }

    pub const fn cedula(&self) -> &Cedula {
        &self.cedula
    }

    pub const fn nombre(&self) -> &NombrePersona {
        &self.nombre
    }

    pub const fn activo(&self) -> bool {
        self.activo
    }

    pub const fn clave(&self) -> &HashClave {
        &self.clave
    }

    pub const fn debe_cambiar_clave(&self) -> bool {
        self.debe_cambiar_clave
    }
}

// --- Inicio de sesión ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorInicioSesion {
    /// Cédula desconocida o clave equivocada: el mismo error, para no
    /// revelar qué cédulas existen.
    #[error("Cédula o clave incorrecta")]
    CredencialesInvalidas,
    #[error("Demasiados intentos fallidos: espere {minutos} minuto(s) e intente de nuevo")]
    Bloqueado { minutos: i64 },
    #[error("Este usuario está desactivado")]
    Desactivado,
    /// La clave la puso otra persona: hasta cambiarla no se opera.
    #[error("Cambie su clave temporal antes de seguir")]
    ClaveTemporal,
}

impl ErrorInicioSesion {
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::CredencialesInvalidas => "credenciales_invalidas",
            Self::Bloqueado { .. } => "inicio_bloqueado",
            Self::Desactivado => "usuario_desactivado",
            Self::ClaveTemporal => "clave_temporal",
        }
    }

    /// Si este rechazo cuenta como intento fallido para el bloqueo (L6):
    /// sí la clave equivocada o la cédula desconocida; no el usuario
    /// desactivado (sabía la clave) ni el que ya estaba bloqueado.
    pub const fn suma_intento(self) -> bool {
        matches!(self, Self::CredencialesInvalidas)
    }
}

/// Intentos fallidos seguidos con una cédula (L6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentosFallidos {
    pub cantidad: u32,
    pub ultimo: DateTime<Utc>,
}

fn ventana() -> TimeDelta {
    TimeDelta::minutes(MINUTOS_DE_BLOQUEO)
}

/// Si la cédula está bloqueada ahora. Se revisa antes de verificar la
/// clave: mientras dure el bloqueo, ni se intenta.
pub fn verificar_bloqueo(
    intentos: Option<IntentosFallidos>,
    ahora: DateTime<Utc>,
) -> Result<(), ErrorInicioSesion> {
    match intentos {
        Some(intentos)
            if intentos.cantidad >= INTENTOS_ANTES_DEL_BLOQUEO
                && ahora < intentos.ultimo + ventana() =>
        {
            let faltan = intentos.ultimo + ventana() - ahora;
            // Redondeado hacia arriba: "1 minuto" aunque falten segundos.
            let minutos = (faltan.num_seconds() + 59) / 60;
            Err(ErrorInicioSesion::Bloqueado { minutos })
        }
        _ => Ok(()),
    }
}

/// Un intento fallido más. Un fallo anterior a la ventana ya no cuenta: se
/// empieza de nuevo.
pub fn sumar_fallo(previos: Option<IntentosFallidos>, ahora: DateTime<Utc>) -> IntentosFallidos {
    let cantidad = match previos {
        Some(previos) if ahora < previos.ultimo + ventana() => previos.cantidad.saturating_add(1),
        _ => 1,
    };
    IntentosFallidos {
        cantidad,
        ultimo: ahora,
    }
}

/// Si quien tiene la sesión puede operar: con una clave temporal (la
/// eligió otra persona) sólo puede cambiarla o cerrar la sesión.
pub const fn puede_operar(debe_cambiar_clave: bool) -> Result<(), ErrorInicioSesion> {
    if debe_cambiar_clave {
        Err(ErrorInicioSesion::ClaveTemporal)
    } else {
        Ok(())
    }
}

/// Decide un inicio de sesión ya verificado: `usuario` es el de la cédula
/// (si existe) y `clave_correcta`, lo que dijo el cifrador. Devuelve
/// el usuario que entra.
pub fn decidir_inicio(
    usuario: Option<&Usuario>,
    clave_correcta: bool,
) -> Result<&Usuario, ErrorInicioSesion> {
    match usuario {
        Some(usuario) if clave_correcta => {
            if usuario.activo() {
                Ok(usuario)
            } else {
                Err(ErrorInicioSesion::Desactivado)
            }
        }
        _ => Err(ErrorInicioSesion::CredencialesInvalidas),
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn cedula() -> Cedula {
        Cedula::normalizar("111111111").unwrap()
    }

    fn id(n: u128) -> OperadorId {
        OperadorId::desde_uuid(Uuid::from_u128(n))
    }

    fn hash(texto: &str) -> HashClave {
        HashClave::desde_texto(texto.to_owned())
    }

    fn ana() -> Usuario {
        Usuario::registrar(
            id(1),
            cedula(),
            "ana mora",
            hash("h1"),
            false,
            HechosUsuario {
                cedula_en_uso: false,
            },
        )
        .unwrap()
    }

    fn en(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    #[test]
    fn la_cedula_de_un_usuario_es_nacional_o_de_extranjero() {
        assert_eq!(
            cedula_de_usuario("1-1111-1111"),
            Ok(cedula()),
            "forma única (A1)"
        );
        assert_eq!(cedula_de_usuario("  "), Err(ErrorUsuario::CedulaVacia));
        assert_eq!(
            cedula_de_usuario("AB123"),
            Err(ErrorUsuario::CedulaInvalida)
        );
    }

    #[test]
    fn la_clave_cumple_l3() {
        assert_eq!(
            ClaveNueva::nueva("corta", &cedula()),
            Err(ErrorUsuario::ClaveCorta)
        );
        assert_eq!(
            ClaveNueva::nueva(&"x".repeat(129), &cedula()),
            Err(ErrorUsuario::ClaveLarga)
        );
        assert_eq!(
            ClaveNueva::nueva("1-1111-1111", &cedula()),
            Err(ErrorUsuario::ClaveIgualALaCedula),
            "la cédula, escrita como sea"
        );
        let buena = ClaveNueva::nueva("ñandú y más", &cedula()).unwrap();
        assert_eq!(
            buena.as_str(),
            "ñandú y más",
            "se cuentan caracteres, no bytes"
        );
        assert!(
            ClaveNueva::nueva(&"á".repeat(128), &cedula()).is_ok(),
            "128 caracteres con tilde caben"
        );
    }

    #[test]
    fn ni_la_clave_ni_su_hash_se_muestran() {
        let texto = format!(
            "{:?} {:?}",
            ClaveNueva::nueva("secreto-123", &cedula()).unwrap(),
            ana()
        );
        assert!(!texto.contains("secreto"), "{texto}");
        assert!(!texto.contains("h1"), "{texto}");
    }

    #[test]
    fn el_alta_valida_el_nombre_y_la_cedula_repetida() {
        let repetida = Usuario::registrar(
            id(2),
            cedula(),
            "beto",
            hash("h"),
            false,
            HechosUsuario {
                cedula_en_uso: true,
            },
        );
        assert_eq!(repetida, Err(ErrorUsuario::CedulaRepetida));
        let sin_nombre = Usuario::registrar(
            id(2),
            cedula(),
            " ",
            hash("h"),
            false,
            HechosUsuario {
                cedula_en_uso: false,
            },
        );
        assert_eq!(sin_nombre, Err(ErrorUsuario::NombreVacio));
        let usuario = ana();
        assert!(usuario.activo(), "nace activo");
        assert_eq!(usuario.nombre().as_str(), "ANA MORA");
        let alta: Vec<&str> = usuario.cambios_de_alta().iter().map(|c| c.campo).collect();
        assert_eq!(alta, ["cedula", "nombre", "activo", "debe_cambiar_clave"]);
    }

    #[test]
    fn el_primer_usuario_solo_se_crea_si_no_hay_ninguno() {
        let primero =
            Usuario::crear_primero(id(1), cedula(), "ana mora", hash("h"), false).unwrap();
        assert!(primero.activo(), "entra activo");
        assert!(!primero.debe_cambiar_clave(), "eligió su clave");
        assert_eq!(
            Usuario::crear_primero(id(2), cedula(), "ana mora", hash("h"), true),
            Err(ErrorUsuario::YaHayUsuarios)
        );
    }

    #[test]
    fn nadie_se_desactiva_a_si_mismo() {
        let mut usuario = ana();
        assert_eq!(
            usuario.editar("ana mora", false, id(1)),
            Err(ErrorUsuario::NoSeDesactivaASiMismo)
        );
        assert!(usuario.activo(), "no cambió nada");
        let cambios = usuario.editar("ana mora", false, id(2)).unwrap();
        assert_eq!(cambios.len(), 1, "otro sí lo desactiva: {cambios:?}");
        assert!(!usuario.activo());
    }

    #[test]
    fn cambiar_la_clave_pide_la_actual_y_restablecerla_la_deja_temporal() {
        let mut usuario = ana();
        assert_eq!(
            usuario.cambiar_clave(false, hash("h2")),
            Err(ErrorUsuario::ClaveActualIncorrecta)
        );
        assert_eq!(usuario.clave(), &hash("h1"), "no cambió");

        let restablecida = usuario.restablecer_clave(hash("h2"));
        assert!(usuario.debe_cambiar_clave(), "queda temporal");
        assert_eq!(restablecida[0].campo, "clave");
        assert_eq!(restablecida[0].despues, "cambiada", "nunca la clave");

        usuario.cambiar_clave(true, hash("h3")).unwrap();
        assert!(!usuario.debe_cambiar_clave(), "ya no es temporal");
        assert_eq!(usuario.clave(), &hash("h3"));
    }

    #[test]
    fn el_inicio_no_revela_si_la_cedula_existe() {
        let mut desactivado = ana();
        desactivado.editar("ana mora", false, id(2)).unwrap();
        let usuario = ana();
        assert_eq!(decidir_inicio(Some(&usuario), true), Ok(&usuario));
        assert_eq!(
            decidir_inicio(Some(&ana()), false),
            Err(ErrorInicioSesion::CredencialesInvalidas)
        );
        assert_eq!(
            decidir_inicio(None, false),
            Err(ErrorInicioSesion::CredencialesInvalidas),
            "igual que una clave equivocada"
        );
        assert_eq!(
            decidir_inicio(Some(&desactivado), false),
            Err(ErrorInicioSesion::CredencialesInvalidas),
            "a quien no sabe la clave no se le dice que está desactivado"
        );
        assert_eq!(
            decidir_inicio(Some(&desactivado), true),
            Err(ErrorInicioSesion::Desactivado)
        );
    }

    #[test]
    fn solo_la_clave_equivocada_suma_intento() {
        assert!(ErrorInicioSesion::CredencialesInvalidas.suma_intento());
        assert!(!ErrorInicioSesion::Desactivado.suma_intento());
        assert!(!ErrorInicioSesion::Bloqueado { minutos: 1 }.suma_intento());
        assert!(!ErrorInicioSesion::ClaveTemporal.suma_intento());
    }

    #[test]
    fn con_clave_temporal_no_se_opera() {
        assert_eq!(puede_operar(false), Ok(()));
        assert_eq!(puede_operar(true), Err(ErrorInicioSesion::ClaveTemporal));
    }

    #[test]
    fn cinco_fallos_seguidos_bloquean_cinco_minutos() {
        let mut intentos = None;
        for minuto in 0..4 {
            intentos = Some(sumar_fallo(
                intentos,
                en(&format!("2026-10-09T08:0{minuto}:00Z")),
            ));
            assert_eq!(
                verificar_bloqueo(intentos, en("2026-10-09T08:04:00Z")),
                Ok(())
            );
        }
        let quinto = Some(sumar_fallo(intentos, en("2026-10-09T08:04:00Z")));
        assert_eq!(quinto.map(|i| i.cantidad), Some(5));
        assert_eq!(
            verificar_bloqueo(quinto, en("2026-10-09T08:04:30Z")),
            Err(ErrorInicioSesion::Bloqueado { minutos: 5 }),
            "faltan 4,5 minutos: se dice 5"
        );
        assert_eq!(
            verificar_bloqueo(quinto, en("2026-10-09T08:08:59Z")),
            Err(ErrorInicioSesion::Bloqueado { minutos: 1 })
        );
        assert_eq!(
            verificar_bloqueo(quinto, en("2026-10-09T08:09:00Z")),
            Ok(()),
            "a los 5 minutos se puede intentar de nuevo"
        );
        let despues = sumar_fallo(quinto, en("2026-10-09T08:09:00Z"));
        assert_eq!(despues.cantidad, 1, "pasado el bloqueo se cuenta de nuevo");
    }

    #[test]
    fn un_fallo_viejo_ya_no_suma() {
        let viejo = sumar_fallo(None, en("2026-10-09T08:00:00Z"));
        assert_eq!(
            sumar_fallo(Some(viejo), en("2026-10-09T08:04:59Z")).cantidad,
            2
        );
        assert_eq!(
            sumar_fallo(Some(viejo), en("2026-10-09T08:05:00Z")).cantidad,
            1
        );
    }

    #[test]
    fn cada_error_tiene_codigo() {
        assert_eq!(
            ErrorUsuario::CedulaRepetida.codigo(),
            "usuario_cedula_repetida"
        );
        assert_eq!(
            ErrorInicioSesion::Bloqueado { minutos: 3 }.codigo(),
            "inicio_bloqueado"
        );
        assert_eq!(
            ErrorInicioSesion::CredencialesInvalidas.to_string(),
            "Cédula o clave incorrecta"
        );
    }
}
