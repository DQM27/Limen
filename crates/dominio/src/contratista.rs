//! Contratista: la persona externa o de planta que entra a trabajar.
//!
//! Reglas de negocio:
//! - cédula obligatoria, nacional o de extranjero (9 a 13 dígitos), en su
//!   forma única, y sin repetir;
//! - nombre obligatorio (ver [`NombrePersona`]);
//! - pertenece a una empresa que existe;
//! - tipo PRAIND o IN HOUSE; los dos exigen el PRAIND al día;
//! - no se registra con un PRAIND vencido; al editar, el vencimiento sólo se
//!   revisa si cambió la fecha (para poder quitarle el acceso o corregirle
//!   el nombre a alguien con el PRAIND vencido);
//! - no se le cambia la cédula mientras está adentro;
//! - todo cambio queda en la auditoría.
//!
//! Las reglas que necesitan datos (cédula repetida, empresa existente,
//! "está adentro") también viven acá: el caso de uso averigua el dato y lo
//! entrega en [`HechosContratista`]; el dominio decide.

use std::fmt;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::auditoria::{CambioCampo, anotar_si_cambia};
use crate::cedula::{Cedula, CedulaInvalida};
use crate::empresa::EmpresaId;
use crate::nombre::{NombreInvalido, NombrePersona};
use crate::praind::praind_vencido;
use crate::tipo_ingreso::TipoIngreso;

/// Identificador global de un contratista (UUID v7). Lo genera quien lo
/// registra, no el dominio. Es distinto de la cédula porque la cédula se
/// puede corregir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContratistaId(Uuid);

impl ContratistaId {
    pub const fn desde_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for ContratistaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Lo que llega del formulario, tal cual se escribió.
#[derive(Debug, Clone, Copy)]
pub struct DatosContratista<'a> {
    pub cedula: &'a str,
    pub nombre: &'a str,
    pub empresa: EmpresaId,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: NaiveDate,
    pub tiene_acceso: bool,
}

/// Lo que el caso de uso averiguó en la base antes de pedirle al dominio
/// que decida.
#[derive(Debug, Clone, Copy)]
pub struct HechosContratista {
    /// Otro contratista ya tiene la cédula (normalizada) que se quiere usar.
    pub cedula_en_uso: bool,
    pub empresa_existe: bool,
    /// El contratista tiene un ingreso abierto ahora mismo. Al registrar
    /// uno nuevo siempre es `false`.
    pub esta_adentro: bool,
}

/// Por qué no se puede guardar. Mismo texto ([`ErrorContratista`] implementa
/// `Display`) y mismo código ([`ErrorContratista::codigo`]) en todas las
/// interfaces. Si fallan varias reglas se informa la primera, en el orden
/// en que aparecen acá.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorContratista {
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    #[error("La cédula debe tener sólo números, entre 9 y 13 dígitos")]
    CedulaInvalida,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("El nombre sólo puede tener letras y espacios")]
    NombreInvalido,
    #[error("La empresa no existe")]
    EmpresaNoExiste,
    #[error("El PRAIND está vencido: ingrese una fecha vigente")]
    PraindVencido,
    #[error("Ya existe un contratista con esa cédula")]
    CedulaRepetida,
    #[error("No se puede cambiar la cédula de alguien que está adentro")]
    CedulaNoEditableAdentro,
}

impl ErrorContratista {
    /// Código estable (no cambia si cambia el texto).
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::CedulaVacia => "cedula_vacia",
            Self::CedulaInvalida => "cedula_invalida",
            Self::NombreVacio => "nombre_vacio",
            Self::NombreInvalido => "nombre_invalido",
            Self::EmpresaNoExiste => "empresa_no_existe",
            Self::PraindVencido => "praind_vencido",
            Self::CedulaRepetida => "cedula_repetida",
            Self::CedulaNoEditableAdentro => "cedula_no_editable_adentro",
        }
    }
}

/// Cédula de un contratista: la forma única, y sólo nacional o de
/// extranjero. El caso de uso la usa para buscar si ya está en uso antes de
/// llamar a [`Contratista::registrar`].
pub fn cedula_de_contratista(texto: &str) -> Result<Cedula, ErrorContratista> {
    match Cedula::normalizar(texto) {
        Ok(cedula) if cedula.es_nacional_o_de_extranjero() => Ok(cedula),
        Err(CedulaInvalida::Vacia) => Err(ErrorContratista::CedulaVacia),
        Ok(_) | Err(_) => Err(ErrorContratista::CedulaInvalida),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contratista {
    id: ContratistaId,
    cedula: Cedula,
    nombre: NombrePersona,
    empresa: EmpresaId,
    tipo_ingreso: TipoIngreso,
    fecha_vencimiento_praind: NaiveDate,
    tiene_acceso: bool,
}

/// Datos de un contratista ya guardado, para reconstruirlo. Los tipos
/// garantizan que cédula y nombre ya pasaron por su normalización; las
/// demás reglas no se vuelven a revisar porque un dato guardado puede haber
/// quedado fuera de ellas con el tiempo (un PRAIND que venció después).
#[derive(Debug, Clone)]
pub struct ContratistaGuardado {
    pub id: ContratistaId,
    pub cedula: Cedula,
    pub nombre: NombrePersona,
    pub empresa: EmpresaId,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: NaiveDate,
    pub tiene_acceso: bool,
}

/// Los datos del formulario ya validados y normalizados.
struct Validados {
    cedula: Cedula,
    nombre: NombrePersona,
}

impl Contratista {
    /// Registra un contratista nuevo aplicando todas las reglas.
    pub fn registrar(
        id: ContratistaId,
        datos: DatosContratista<'_>,
        hechos: HechosContratista,
        hoy: NaiveDate,
    ) -> Result<Self, ErrorContratista> {
        let Validados { cedula, nombre } = validar_formato(&datos)?;
        if !hechos.empresa_existe {
            return Err(ErrorContratista::EmpresaNoExiste);
        }
        if praind_vencido(datos.fecha_vencimiento_praind, hoy) {
            return Err(ErrorContratista::PraindVencido);
        }
        if hechos.cedula_en_uso {
            return Err(ErrorContratista::CedulaRepetida);
        }
        Ok(Self {
            id,
            cedula,
            nombre,
            empresa: datos.empresa,
            tipo_ingreso: datos.tipo_ingreso,
            fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
            tiene_acceso: datos.tiene_acceso,
        })
    }

    /// Edita el contratista y devuelve todo lo que cambió, para la
    /// auditoría. Si alguna regla falla, el contratista queda como estaba.
    pub fn editar(
        &mut self,
        datos: DatosContratista<'_>,
        hechos: HechosContratista,
        hoy: NaiveDate,
    ) -> Result<Vec<CambioCampo>, ErrorContratista> {
        let Validados { cedula, nombre } = validar_formato(&datos)?;
        if !hechos.empresa_existe {
            return Err(ErrorContratista::EmpresaNoExiste);
        }
        let cambia_fecha = datos.fecha_vencimiento_praind != self.fecha_vencimiento_praind;
        if cambia_fecha && praind_vencido(datos.fecha_vencimiento_praind, hoy) {
            return Err(ErrorContratista::PraindVencido);
        }
        let cambia_cedula = cedula != self.cedula;
        if cambia_cedula && hechos.cedula_en_uso {
            return Err(ErrorContratista::CedulaRepetida);
        }
        if cambia_cedula && hechos.esta_adentro {
            return Err(ErrorContratista::CedulaNoEditableAdentro);
        }

        let mut cambios = Vec::new();
        anotar_si_cambia(&mut cambios, "cedula", &self.cedula, &cedula);
        anotar_si_cambia(&mut cambios, "nombre", &self.nombre, &nombre);
        anotar_si_cambia(&mut cambios, "empresa", &self.empresa, &datos.empresa);
        anotar_si_cambia(
            &mut cambios,
            "tipo_ingreso",
            &self.tipo_ingreso,
            &datos.tipo_ingreso,
        );
        anotar_si_cambia(
            &mut cambios,
            "fecha_vencimiento_praind",
            &self.fecha_vencimiento_praind,
            &datos.fecha_vencimiento_praind,
        );
        anotar_si_cambia(
            &mut cambios,
            "tiene_acceso",
            &self.tiene_acceso,
            &datos.tiene_acceso,
        );

        self.cedula = cedula;
        self.nombre = nombre;
        self.empresa = datos.empresa;
        self.tipo_ingreso = datos.tipo_ingreso;
        self.fecha_vencimiento_praind = datos.fecha_vencimiento_praind;
        self.tiene_acceso = datos.tiene_acceso;
        Ok(cambios)
    }

    pub fn restaurar(guardado: ContratistaGuardado) -> Self {
        Self {
            id: guardado.id,
            cedula: guardado.cedula,
            nombre: guardado.nombre,
            empresa: guardado.empresa,
            tipo_ingreso: guardado.tipo_ingreso,
            fecha_vencimiento_praind: guardado.fecha_vencimiento_praind,
            tiene_acceso: guardado.tiene_acceso,
        }
    }

    pub const fn id(&self) -> ContratistaId {
        self.id
    }

    pub const fn cedula(&self) -> &Cedula {
        &self.cedula
    }

    pub const fn nombre(&self) -> &NombrePersona {
        &self.nombre
    }

    pub const fn empresa(&self) -> EmpresaId {
        self.empresa
    }

    pub const fn tipo_ingreso(&self) -> TipoIngreso {
        self.tipo_ingreso
    }

    pub const fn fecha_vencimiento_praind(&self) -> NaiveDate {
        self.fecha_vencimiento_praind
    }

    pub const fn tiene_acceso(&self) -> bool {
        self.tiene_acceso
    }

    /// Sólo PRAIND recibe gafete físico; para IN HOUSE su credencial ya es
    /// el gafete.
    pub const fn requiere_gafete(&self) -> bool {
        self.tipo_ingreso.requiere_gafete()
    }
}

fn validar_formato(datos: &DatosContratista<'_>) -> Result<Validados, ErrorContratista> {
    let cedula = cedula_de_contratista(datos.cedula)?;
    let nombre = NombrePersona::nuevo(datos.nombre).map_err(|error| match error {
        NombreInvalido::Vacio => ErrorContratista::NombreVacio,
        NombreInvalido::CaracteresNoPermitidos => ErrorContratista::NombreInvalido,
    })?;
    Ok(Validados { cedula, nombre })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    fn hoy() -> NaiveDate {
        fecha("2026-10-09")
    }

    fn empresa() -> EmpresaId {
        EmpresaId::desde_uuid(Uuid::from_u128(1))
    }

    fn datos() -> DatosContratista<'static> {
        DatosContratista {
            cedula: "1-1111-1111",
            nombre: "  josé  peña ",
            empresa: empresa(),
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: fecha("2027-01-01"),
            tiene_acceso: true,
        }
    }

    const fn hechos() -> HechosContratista {
        HechosContratista {
            cedula_en_uso: false,
            empresa_existe: true,
            esta_adentro: false,
        }
    }

    fn registrado() -> Contratista {
        Contratista::registrar(
            ContratistaId::desde_uuid(Uuid::from_u128(9)),
            datos(),
            hechos(),
            hoy(),
        )
        .unwrap()
    }

    fn registrar(
        datos: DatosContratista<'_>,
        hechos: HechosContratista,
    ) -> Result<Contratista, ErrorContratista> {
        Contratista::registrar(
            ContratistaId::desde_uuid(Uuid::from_u128(9)),
            datos,
            hechos,
            hoy(),
        )
    }

    #[test]
    fn registra_normalizando_cedula_y_nombre() {
        let contratista = registrado();
        assert_eq!(contratista.cedula().as_str(), "111111111");
        assert_eq!(contratista.nombre().as_str(), "JOSE PEÑA");
        assert!(contratista.requiere_gafete());
    }

    #[test]
    fn in_house_no_lleva_gafete() {
        let mut in_house = datos();
        in_house.tipo_ingreso = TipoIngreso::InHouse;
        assert!(!registrar(in_house, hechos()).unwrap().requiere_gafete());
    }

    #[test]
    fn informa_el_primer_motivo_en_orden() {
        let mut d = datos();
        d.cedula = " ";
        d.nombre = "";
        assert_eq!(registrar(d, hechos()), Err(ErrorContratista::CedulaVacia));

        let mut pasaporte = datos();
        pasaporte.cedula = "AB123456";
        assert_eq!(
            registrar(pasaporte, hechos()),
            Err(ErrorContratista::CedulaInvalida)
        );

        let mut sin_nombre = datos();
        sin_nombre.nombre = "  ";
        assert_eq!(
            registrar(sin_nombre, hechos()),
            Err(ErrorContratista::NombreVacio)
        );

        let mut con_numero = datos();
        con_numero.nombre = "Ana 2";
        assert_eq!(
            registrar(con_numero, hechos()),
            Err(ErrorContratista::NombreInvalido)
        );
    }

    #[test]
    fn la_empresa_debe_existir() {
        let sin_empresa = HechosContratista {
            empresa_existe: false,
            ..hechos()
        };
        assert_eq!(
            registrar(datos(), sin_empresa),
            Err(ErrorContratista::EmpresaNoExiste)
        );
    }

    #[test]
    fn no_se_registra_con_praind_vencido_pero_vencer_hoy_es_valido() {
        let mut vencido = datos();
        vencido.fecha_vencimiento_praind = fecha("2026-10-08");
        assert_eq!(
            registrar(vencido, hechos()),
            Err(ErrorContratista::PraindVencido)
        );

        let mut vence_hoy = datos();
        vence_hoy.fecha_vencimiento_praind = hoy();
        assert!(registrar(vence_hoy, hechos()).is_ok());
    }

    #[test]
    fn la_cedula_no_se_repite() {
        let repetida = HechosContratista {
            cedula_en_uso: true,
            ..hechos()
        };
        assert_eq!(
            registrar(datos(), repetida),
            Err(ErrorContratista::CedulaRepetida)
        );
    }

    #[test]
    fn editar_audita_todos_los_campos_que_cambian() {
        let mut contratista = registrado();
        let mut nuevos = datos();
        nuevos.nombre = "jose pena";
        nuevos.tiene_acceso = false;
        let cambios = contratista.editar(nuevos, hechos(), hoy()).unwrap();
        let campos: Vec<_> = cambios.iter().map(|cambio| cambio.campo).collect();
        assert_eq!(campos, ["nombre", "tiene_acceso"]);
        assert_eq!(cambios[0].antes, "JOSE PEÑA");
        assert_eq!(cambios[0].despues, "JOSE PENA");
        assert!(!contratista.tiene_acceso());
    }

    #[test]
    fn editar_sin_cambios_no_audita_nada() {
        let mut contratista = registrado();
        assert_eq!(contratista.editar(datos(), hechos(), hoy()), Ok(vec![]));
    }

    #[test]
    fn con_praind_vencido_se_puede_editar_si_no_cambia_la_fecha() {
        let mut contratista = registrado();
        let mas_tarde = fecha("2027-06-01");
        let mut quitar_acceso = datos();
        quitar_acceso.tiene_acceso = false;
        assert!(
            contratista
                .editar(quitar_acceso, hechos(), mas_tarde)
                .is_ok()
        );

        let mut fecha_vencida = datos();
        fecha_vencida.fecha_vencimiento_praind = fecha("2027-05-01");
        assert_eq!(
            contratista.editar(fecha_vencida, hechos(), mas_tarde),
            Err(ErrorContratista::PraindVencido)
        );
    }

    #[test]
    fn no_cambia_la_cedula_de_quien_esta_adentro() {
        let mut contratista = registrado();
        let adentro = HechosContratista {
            esta_adentro: true,
            ..hechos()
        };
        let mut otra_cedula = datos();
        otra_cedula.cedula = "222222222";
        assert_eq!(
            contratista.editar(otra_cedula, adentro, hoy()),
            Err(ErrorContratista::CedulaNoEditableAdentro)
        );
        assert_eq!(
            contratista.cedula().as_str(),
            "111111111",
            "quedó como estaba"
        );

        let mut otro_nombre = datos();
        otro_nombre.nombre = "ana solano";
        assert!(
            contratista.editar(otro_nombre, adentro, hoy()).is_ok(),
            "el resto sí se edita"
        );
    }

    #[test]
    fn su_propia_cedula_no_cuenta_como_repetida() {
        let mut contratista = registrado();
        let en_uso = HechosContratista {
            cedula_en_uso: true,
            ..hechos()
        };
        assert!(contratista.editar(datos(), en_uso, hoy()).is_ok());
    }

    #[test]
    fn cada_motivo_tiene_codigo_estable() {
        assert_eq!(ErrorContratista::PraindVencido.codigo(), "praind_vencido");
        assert_eq!(
            ErrorContratista::CedulaInvalida.to_string(),
            "La cédula debe tener sólo números, entre 9 y 13 dígitos"
        );
    }
}
