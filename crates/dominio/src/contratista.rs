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
//! - a quien se registra con el acceso denegado no se le pide el PRAIND (A8:
//!   se le vetó, no va a entrar); si luego se le devuelve el acceso, el
//!   PRAIND sí tiene que estar al día;
//! - no se le cambia la cédula mientras está adentro;
//! - todo cambio queda en la auditoría.
//!
//! Las reglas que necesitan datos (cédula repetida, empresa existente,
//! "está adentro") también viven acá: el caso de uso averigua el dato y lo
//! entrega en [`HechosContratista`]; el dominio decide.

use std::fmt;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::auditoria::{CambioCampo, CamposAuditables, cambios_de_alta, diferencias};
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

/// La fecha de PRAIND de quien se registró con el acceso denegado y sin PRAIND:
/// un día que ya pasó, así que vale como vencido si alguna vez se le devuelve
/// el acceso sin ponerle uno.
pub const SIN_PRAIND: NaiveDate = match NaiveDate::from_ymd_opt(1970, 1, 1) {
    Some(fecha) => fecha,
    None => panic!("fecha válida"),
};

/// ¿Esta fecha es un PRAIND de verdad, o la de quien nunca tuvo uno?
pub fn tiene_praind(fecha: NaiveDate) -> bool {
    fecha != SIN_PRAIND
}

/// Lo que llega del formulario, tal cual se escribió. La fecha del PRAIND puede
/// faltar sólo si no se le da el acceso.
#[derive(Debug, Clone, Copy)]
pub struct DatosContratista<'a> {
    pub cedula: &'a str,
    pub nombre: &'a str,
    pub empresa: EmpresaId,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub tiene_acceso: bool,
}

/// Lo que hay en el formulario mientras todavía puede estar incompleto: sin
/// empresa elegida o sin fecha de PRAIND. Sirve para saber de una vez qué
/// falta y qué está mal (ver [`problemas`]). El tipo de ingreso siempre tiene
/// un valor válido, así que no entra acá; el acceso sí, porque sin acceso no
/// se pide el PRAIND.
#[derive(Debug, Clone, Copy)]
pub struct BorradorContratista<'a> {
    pub cedula: &'a str,
    pub nombre: &'a str,
    pub empresa: Option<EmpresaId>,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
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
/// interfaces. [`Contratista::registrar`] y [`Contratista::editar`] informan
/// el primer motivo, en el orden en que aparecen acá; [`problemas`] los
/// informa todos, para que el formulario marque cada campo a la vez.
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
    #[error("La empresa es obligatoria")]
    EmpresaRequerida,
    #[error("La empresa no existe")]
    EmpresaNoExiste,
    #[error("La fecha de vencimiento del PRAIND es obligatoria")]
    PraindRequerido,
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
            Self::EmpresaRequerida => "empresa_requerida",
            Self::EmpresaNoExiste => "empresa_no_existe",
            Self::PraindRequerido => "praind_requerido",
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

/// Todo lo que impide guardar este formulario, en el orden de
/// [`ErrorContratista`] (vacío si se puede guardar). `anterior` es el
/// contratista que se edita, si es una edición: la fecha del PRAIND sólo se
/// revisa si cambió (para poder quitar el acceso o corregir el nombre de
/// alguien con el PRAIND vencido) y la cédula sólo choca si cambió.
pub fn problemas(
    borrador: &BorradorContratista<'_>,
    hechos: HechosContratista,
    hoy: NaiveDate,
    anterior: Option<&Contratista>,
) -> Vec<ErrorContratista> {
    let mut problemas = Vec::new();
    let cedula = cedula_de_contratista(borrador.cedula);
    if let Err(error) = cedula {
        problemas.push(error);
    }
    if let Err(error) = NombrePersona::nuevo(borrador.nombre) {
        problemas.push(match error {
            NombreInvalido::Vacio => ErrorContratista::NombreVacio,
            NombreInvalido::CaracteresNoPermitidos => ErrorContratista::NombreInvalido,
        });
    }
    match borrador.empresa {
        None => problemas.push(ErrorContratista::EmpresaRequerida),
        Some(_) if !hechos.empresa_existe => problemas.push(ErrorContratista::EmpresaNoExiste),
        Some(_) => {}
    }
    match borrador.fecha_vencimiento_praind {
        // Sin acceso no se pide el PRAIND: no va a entrar.
        None if !borrador.tiene_acceso => {}
        None => problemas.push(ErrorContratista::PraindRequerido),
        Some(fecha) => {
            // Se revisa si es nuevo, si cambió la fecha o si se le devuelve el acceso.
            let cambia = anterior.is_none_or(|a| {
                a.fecha_vencimiento_praind != fecha || (!a.tiene_acceso && borrador.tiene_acceso)
            });
            if borrador.tiene_acceso && cambia && praind_vencido(fecha, hoy) {
                problemas.push(ErrorContratista::PraindVencido);
            }
        }
    }
    if let Ok(cedula) = cedula {
        let cambia = anterior.is_none_or(|a| a.cedula != cedula);
        if cambia && hechos.cedula_en_uso {
            problemas.push(ErrorContratista::CedulaRepetida);
        }
        if anterior.is_some() && cambia && hechos.esta_adentro {
            problemas.push(ErrorContratista::CedulaNoEditableAdentro);
        }
    }
    problemas
}

/// El primer motivo por el que no se puede guardar, si hay alguno.
fn primer_problema(
    datos: &DatosContratista<'_>,
    hechos: HechosContratista,
    hoy: NaiveDate,
    anterior: Option<&Contratista>,
) -> Result<(), ErrorContratista> {
    let borrador = BorradorContratista {
        cedula: datos.cedula,
        nombre: datos.nombre,
        empresa: Some(datos.empresa),
        fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
        tiene_acceso: datos.tiene_acceso,
    };
    problemas(&borrador, hechos, hoy, anterior)
        .into_iter()
        .next()
        .map_or(Ok(()), Err)
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
        primer_problema(&datos, hechos, hoy, None)?;
        let Validados { cedula, nombre } = validar_formato(&datos)?;
        Ok(Self {
            id,
            cedula,
            nombre,
            empresa: datos.empresa,
            tipo_ingreso: datos.tipo_ingreso,
            fecha_vencimiento_praind: datos.fecha_vencimiento_praind.unwrap_or(SIN_PRAIND),
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
        primer_problema(&datos, hechos, hoy, Some(self))?;
        let Validados { cedula, nombre } = validar_formato(&datos)?;

        let antes = self.campos_auditables();
        self.cedula = cedula;
        self.nombre = nombre;
        self.empresa = datos.empresa;
        self.tipo_ingreso = datos.tipo_ingreso;
        // Si no se da el PRAIND (sólo sin acceso) queda el que tenía.
        self.fecha_vencimiento_praind = datos
            .fecha_vencimiento_praind
            .unwrap_or(self.fecha_vencimiento_praind);
        self.tiene_acceso = datos.tiene_acceso;
        Ok(diferencias(antes, self.campos_auditables()))
    }

    /// Lo que la auditoría registra del alta: todos los campos.
    pub fn cambios_de_alta(&self) -> Vec<CambioCampo> {
        cambios_de_alta(self.campos_auditables())
    }

    /// Todos los campos que la auditoría vigila, en un orden fijo. Un campo
    /// nuevo del contratista se agrega acá y queda auditado en el alta y en
    /// cada edición.
    fn campos_auditables(&self) -> CamposAuditables {
        vec![
            ("cedula", self.cedula.to_string()),
            ("nombre", self.nombre.to_string()),
            ("empresa", self.empresa.to_string()),
            ("tipo_ingreso", self.tipo_ingreso.to_string()),
            (
                "fecha_vencimiento_praind",
                self.fecha_vencimiento_praind.to_string(),
            ),
            ("tiene_acceso", self.tiene_acceso.to_string()),
        ]
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
            fecha_vencimiento_praind: Some(fecha("2027-01-01")),
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
        vencido.fecha_vencimiento_praind = Some(fecha("2026-10-08"));
        assert_eq!(
            registrar(vencido, hechos()),
            Err(ErrorContratista::PraindVencido)
        );

        let mut vence_hoy = datos();
        vence_hoy.fecha_vencimiento_praind = Some(hoy());
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

    fn borrador() -> BorradorContratista<'static> {
        BorradorContratista {
            cedula: "1-1111-1111",
            nombre: "josé peña",
            empresa: Some(empresa()),
            fecha_vencimiento_praind: Some(fecha("2027-01-01")),
            tiene_acceso: true,
        }
    }

    #[test]
    fn un_formulario_completo_y_correcto_no_tiene_problemas() {
        assert_eq!(problemas(&borrador(), hechos(), hoy(), None), []);
    }

    #[test]
    fn un_formulario_vacio_marca_todos_los_campos_a_la_vez() {
        let vacio = BorradorContratista {
            cedula: "",
            nombre: " ",
            empresa: None,
            fecha_vencimiento_praind: None,
            tiene_acceso: true,
        };
        assert_eq!(
            problemas(&vacio, hechos(), hoy(), None),
            [
                ErrorContratista::CedulaVacia,
                ErrorContratista::NombreVacio,
                ErrorContratista::EmpresaRequerida,
                ErrorContratista::PraindRequerido,
            ]
        );
    }

    #[test]
    fn un_dato_malo_no_tapa_a_los_demas() {
        let malo = BorradorContratista {
            cedula: "AB123",
            nombre: "Ana 2",
            empresa: Some(empresa()),
            fecha_vencimiento_praind: Some(fecha("2026-10-08")),
            tiene_acceso: true,
        };
        let sin_empresa = HechosContratista {
            empresa_existe: false,
            ..hechos()
        };
        assert_eq!(
            problemas(&malo, sin_empresa, hoy(), None),
            [
                ErrorContratista::CedulaInvalida,
                ErrorContratista::NombreInvalido,
                ErrorContratista::EmpresaNoExiste,
                ErrorContratista::PraindVencido,
            ]
        );
    }

    #[test]
    fn la_cedula_repetida_se_informa_junto_a_los_demas_problemas() {
        let sin_nombre = BorradorContratista {
            nombre: "",
            ..borrador()
        };
        let repetida = HechosContratista {
            cedula_en_uso: true,
            ..hechos()
        };
        assert_eq!(
            problemas(&sin_nombre, repetida, hoy(), None),
            [
                ErrorContratista::NombreVacio,
                ErrorContratista::CedulaRepetida
            ]
        );
    }

    #[test]
    fn registrar_informa_el_mismo_primer_problema_que_la_lista() {
        let mut d = datos();
        d.cedula = "";
        d.nombre = "";
        assert_eq!(registrar(d, hechos()), Err(ErrorContratista::CedulaVacia));
    }

    #[test]
    fn al_editar_lo_que_no_cambia_no_se_vuelve_a_revisar() {
        let contratista = registrado();
        // Un año después el PRAIND ya venció, pero si no se toca la fecha ni
        // la cédula se puede corregir lo demás.
        let despues = fecha("2028-01-01");
        let chocan = HechosContratista {
            cedula_en_uso: true,
            esta_adentro: true,
            ..hechos()
        };
        assert_eq!(
            problemas(&borrador(), chocan, despues, Some(&contratista)),
            []
        );

        let otra_cedula = BorradorContratista {
            cedula: "222222222",
            ..borrador()
        };
        assert_eq!(
            problemas(&otra_cedula, chocan, despues, Some(&contratista)),
            [
                ErrorContratista::CedulaRepetida,
                ErrorContratista::CedulaNoEditableAdentro
            ]
        );

        let otra_fecha = BorradorContratista {
            fecha_vencimiento_praind: Some(fecha("2027-12-31")),
            ..borrador()
        };
        assert_eq!(
            problemas(&otra_fecha, hechos(), despues, Some(&contratista)),
            [ErrorContratista::PraindVencido]
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
    fn el_alta_audita_todos_los_campos() {
        let cambios = registrado().cambios_de_alta();
        let campos: Vec<_> = cambios.iter().map(|cambio| cambio.campo).collect();
        assert_eq!(
            campos,
            [
                "cedula",
                "nombre",
                "empresa",
                "tipo_ingreso",
                "fecha_vencimiento_praind",
                "tiene_acceso"
            ],
            "el alta registra cada campo"
        );
        assert_eq!(cambios[0].despues, "111111111", "con su valor normalizado");
        assert_eq!(
            cambios[3].despues, "PRAIND",
            "el tipo con su código estable"
        );
    }

    #[test]
    fn un_error_al_editar_no_cambia_nada() {
        let mut contratista = registrado();
        let antes = contratista.clone();
        let mut invalidos = datos();
        invalidos.nombre = "otro nombre";
        invalidos.fecha_vencimiento_praind = Some(fecha("2020-01-01"));
        assert_eq!(
            contratista.editar(invalidos, hechos(), hoy()),
            Err(ErrorContratista::PraindVencido),
            "la fecha vencida frena la edición"
        );
        assert_eq!(contratista, antes, "ni el nombre se aplicó");
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
        fecha_vencida.fecha_vencimiento_praind = Some(fecha("2027-05-01"));
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

    fn sin_acceso_ni_praind() -> DatosContratista<'static> {
        DatosContratista {
            fecha_vencimiento_praind: None,
            tiene_acceso: false,
            ..datos()
        }
    }

    #[test]
    fn a_quien_se_registra_sin_acceso_no_se_le_pide_el_praind() {
        let vetado = registrar(sin_acceso_ni_praind(), hechos()).unwrap();
        assert!(!vetado.tiene_acceso());
        assert_eq!(vetado.fecha_vencimiento_praind(), SIN_PRAIND);
        assert!(!tiene_praind(vetado.fecha_vencimiento_praind()));
        assert!(tiene_praind(fecha("2027-01-01")));

        let con_acceso = DatosContratista {
            tiene_acceso: true,
            ..sin_acceso_ni_praind()
        };
        assert_eq!(
            registrar(con_acceso, hechos()),
            Err(ErrorContratista::PraindRequerido),
            "con acceso sí hace falta"
        );
    }

    #[test]
    fn sin_acceso_tampoco_importa_que_el_praind_este_vencido() {
        let vencido = DatosContratista {
            fecha_vencimiento_praind: Some(fecha("2020-01-01")),
            tiene_acceso: false,
            ..datos()
        };
        assert!(registrar(vencido, hechos()).is_ok());
    }

    #[test]
    fn al_devolver_el_acceso_el_praind_tiene_que_estar_al_dia() {
        let mut vetado = registrar(sin_acceso_ni_praind(), hechos()).unwrap();

        let sin_fecha = DatosContratista {
            fecha_vencimiento_praind: None,
            ..datos()
        };
        assert_eq!(
            vetado.editar(sin_fecha, hechos(), hoy()),
            Err(ErrorContratista::PraindRequerido)
        );

        let con_la_vieja = DatosContratista {
            fecha_vencimiento_praind: Some(SIN_PRAIND),
            ..datos()
        };
        assert_eq!(
            vetado.editar(con_la_vieja, hechos(), hoy()),
            Err(ErrorContratista::PraindVencido),
            "devolverle el acceso con un PRAIND vencido no se puede"
        );

        assert!(vetado.editar(datos(), hechos(), hoy()).is_ok());
        assert!(vetado.tiene_acceso());
        assert_eq!(vetado.fecha_vencimiento_praind(), fecha("2027-01-01"));
    }

    #[test]
    fn quitar_el_acceso_sin_dar_fecha_conserva_el_praind() {
        let mut contratista = registrado();
        let quitar = DatosContratista {
            fecha_vencimiento_praind: None,
            tiene_acceso: false,
            ..datos()
        };
        let cambios = contratista.editar(quitar, hechos(), hoy()).unwrap();
        let campos: Vec<_> = cambios.iter().map(|cambio| cambio.campo).collect();
        assert_eq!(campos, ["tiene_acceso"], "la fecha no se borra");
        assert_eq!(contratista.fecha_vencimiento_praind(), fecha("2027-01-01"));
    }

    #[test]
    fn cada_motivo_tiene_codigo_estable() {
        assert_eq!(ErrorContratista::PraindVencido.codigo(), "praind_vencido");
        assert_eq!(
            ErrorContratista::EmpresaRequerida.codigo(),
            "empresa_requerida"
        );
        assert_eq!(
            ErrorContratista::PraindRequerido.codigo(),
            "praind_requerido"
        );
        assert_eq!(
            ErrorContratista::CedulaInvalida.to_string(),
            "La cédula debe tener sólo números, entre 9 y 13 dígitos"
        );
    }
}
