//! Casos de uso de ingresos y salidas de contratistas.

use limen_dominio::acceso::ResultadoAcceso;
use limen_dominio::busqueda::Criterio;
use limen_dominio::contratista::{Contratista, ContratistaId};
use limen_dominio::empresa::NombreEmpresa;
use limen_dominio::gafete::{ErrorPrestamoGafete, NumeroGafete, Portador, TipoGafete};
use limen_dominio::hecho::{Hecho, HechoId};
use limen_dominio::ingreso_contratista::{
    DatosEntrada, ErrorIngreso, GafeteIndicado, HechosEntrada, IngresoContratista, IngresoId,
    gafete_que_aplica, puede_entrar,
};
use limen_dominio::medio::TipoMedio;
use limen_dominio::movimiento::{ErrorSalida, Marca};
use limen_dominio::presencia::{Identidad, Via, YaEstaAdentro};

use super::consultas::{ErrorConsulta, limitar};
use super::gafetes::situacion_para_prestar;
use crate::errores::ErrorCaso;
use crate::puertos::{
    Consultas, FabricaUnidadDeTrabajo, GeneradorIds, RegistroHechos, Reloj,
    RepositorioContratistas, RepositorioEmpresas, RepositorioGafetes, RepositorioIngresos,
    RepositorioPresencias, RepositorioReloj, Restriccion, ResumenGafete, UnidadDeTrabajo,
};
use crate::sesion::Sesion;

pub type ErrorEntrada = ErrorCaso<ErrorIngreso>;
pub type ErrorDeSalida = ErrorCaso<ErrorSalida>;

/// La base rechazó la entrada porque otro equipo registró a la misma
/// persona o prestó el mismo gafete primero.
fn conflicto_de_entrada(restriccion: Restriccion) -> Option<ErrorIngreso> {
    match restriccion {
        Restriccion::PresenciaPersona => {
            Some(ErrorIngreso::YaEstaAdentro(YaEstaAdentro(Via::Contratista)))
        }
        Restriccion::GafetePrestado => Some(ErrorIngreso::Gafete(ErrorPrestamoGafete::Prestado)),
        _ => None,
    }
}

/// Lo que el operador eligió sobre el gafete, tal cual llega.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GafeteElegido {
    Numero(u32),
    /// "Sin gafete" (S/G), marcado a propósito.
    SinGafete,
}

/// Lo que llega del formulario de entrada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComandoEntrada {
    pub contratista: ContratistaId,
    pub medio: TipoMedio,
    pub placa: Option<String>,
    /// `None` = el operador no eligió nada: a quien le corresponde gafete
    /// el dominio se lo exige (E3).
    pub gafete: Option<GafeteElegido>,
}

/// La entrada registrada y el resultado del acceso (para avisar si el
/// PRAIND está por vencer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaContratista {
    pub ingreso: IngresoId,
    pub acceso: ResultadoAcceso,
}

#[derive(Debug)]
pub struct RegistrarEntrada<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarEntrada<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    pub async fn ejecutar(
        &self,
        sesion: &Sesion,
        comando: &ComandoEntrada,
    ) -> Result<EntradaContratista, ErrorEntrada> {
        let mut uow = self.fabrica.nueva();
        let contratista = uow
            .contratistas()
            .obtener(comando.contratista)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        // Un gafete 0 no existe en ningún catálogo.
        let gafete = match comando.gafete {
            Some(GafeteElegido::Numero(numero)) => Some(GafeteIndicado::Numero(
                NumeroGafete::nuevo(numero).map_err(|_| {
                    ErrorCaso::Negocio(ErrorIngreso::Gafete(ErrorPrestamoGafete::NoRegistrado))
                })?,
            )),
            Some(GafeteElegido::SinGafete) => Some(GafeteIndicado::SinGafete),
            None => None,
        };

        let situacion_gafete = match gafete_que_aplica(&contratista, gafete) {
            Some(numero) => {
                Some(situacion_para_prestar(uow.gafetes(), TipoGafete::Contratista, numero).await?)
            }
            None => None,
        };
        let hechos = HechosEntrada {
            ultimo_movimiento: uow.reloj().ultimo_movimiento().await?,
            adentro_por: uow
                .presencias()
                .via_adentro(&Identidad::from(contratista.cedula()))
                .await?,
            situacion_gafete,
        };
        let marca = Marca {
            en: self.reloj.ahora(),
            operador: sesion.operador(),
        };
        let datos = DatosEntrada {
            medio: comando.medio,
            placa: comando.placa.as_deref(),
            gafete,
        };
        let registrada = IngresoContratista::registrar_entrada(
            IngresoId::desde_uuid(self.ids.nuevo()),
            &contratista,
            datos,
            hechos,
            marca,
            self.reloj.hoy(),
        )
        .map_err(ErrorCaso::Negocio)?;
        let ingreso = registrada.ingreso;

        uow.ingresos().guardar(&ingreso);
        uow.hechos().anotar(Hecho::entrada_contratista(
            HechoId::desde_uuid(self.ids.nuevo()),
            &ingreso,
        ));
        uow.presencias().anotar_entrada(
            &Identidad::from(ingreso.cedula()),
            Via::Contratista,
            marca.en,
        );
        if let Some(numero) = ingreso.gafete() {
            uow.gafetes()
                .anotar_prestamo(TipoGafete::Contratista, numero, marca.en);
        }
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar()
            .await
            .map_err(|error| ErrorCaso::al_confirmar(error, conflicto_de_entrada))?;
        Ok(EntradaContratista {
            ingreso: ingreso.id(),
            acceso: registrada.acceso,
        })
    }
}

/// Registra la salida de un contratista: por el ingreso (desde la lista de
/// quienes están adentro) o por el número del gafete que devuelve.
#[derive(Debug)]
pub struct RegistrarSalida<F, R, G> {
    fabrica: F,
    reloj: R,
    ids: G,
}

impl<F: FabricaUnidadDeTrabajo, R: Reloj, G: GeneradorIds> RegistrarSalida<F, R, G> {
    pub const fn new(fabrica: F, reloj: R, ids: G) -> Self {
        Self {
            fabrica,
            reloj,
            ids,
        }
    }

    pub async fn ejecutar(&self, sesion: &Sesion, id: IngresoId) -> Result<(), ErrorDeSalida> {
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    /// Salida por el gafete que devuelve la persona.
    pub async fn por_gafete(&self, sesion: &Sesion, numero: u32) -> Result<(), ErrorDeSalida> {
        let numero = NumeroGafete::nuevo(numero).map_err(|_| ErrorCaso::NoEncontrado)?;
        let mut uow = self.fabrica.nueva();
        let ingreso = uow
            .ingresos()
            .abierto_con_gafete(numero)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        self.cerrar(sesion, uow, ingreso).await
    }

    async fn cerrar<U: UnidadDeTrabajo>(
        &self,
        sesion: &Sesion,
        mut uow: U,
        mut ingreso: IngresoContratista,
    ) -> Result<(), ErrorDeSalida> {
        let ultimo_movimiento = uow.reloj().ultimo_movimiento().await?;
        let marca = Marca {
            en: self.reloj.ahora(),
            operador: sesion.operador(),
        };
        ingreso
            .registrar_salida(marca, ultimo_movimiento)
            .map_err(ErrorCaso::Negocio)?;

        uow.ingresos().guardar(&ingreso);
        if let Some(hecho) =
            Hecho::salida_contratista(HechoId::desde_uuid(self.ids.nuevo()), &ingreso)
        {
            uow.hechos().anotar(hecho);
        }
        uow.presencias()
            .anotar_salida(&Identidad::from(ingreso.cedula()));
        if let Some(numero) = ingreso.gafete() {
            uow.gafetes()
                .anotar_devolucion(TipoGafete::Contratista, numero);
        }
        uow.reloj().anotar_movimiento(marca.en);
        uow.confirmar().await.map_err(ErrorCaso::from)
    }
}

/// Lo que la pantalla de ingreso muestra de un contratista antes de
/// registrar la entrada. Todo viene decidido: la pantalla sólo lo muestra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatoIngreso {
    pub contratista: Contratista,
    pub empresa: Option<NombreEmpresa>,
    /// Si puede entrar hoy y con qué aviso, o por qué no: la misma regla
    /// que aplica el registro ([`puede_entrar`]), así nunca se contradicen.
    pub decision: Result<ResultadoAcceso, ErrorIngreso>,
    /// Gafetes de contratista perdidos cuyo último portador es esta persona.
    /// Sólo informa: no impide la entrada.
    pub gafetes_perdidos: Vec<NumeroGafete>,
}

/// Prepara la entrada de un contratista: el buscador (por cédula o por
/// nombre, en el mismo campo) y la ficha del elegido, con la decisión ya
/// tomada por el dominio. Sólo lee: no registra nada.
#[derive(Debug)]
pub struct PrepararIngreso<F, R> {
    fabrica: F,
    reloj: R,
}

impl<F: FabricaUnidadDeTrabajo + Consultas, R: Reloj> PrepararIngreso<F, R> {
    pub const fn new(fabrica: F, reloj: R) -> Self {
        Self { fabrica, reloj }
    }

    /// Los contratistas que cumplen el texto (sólo números: cédula; letras:
    /// nombre), del mejor al peor resultado, cada uno con su decisión. Un
    /// texto vacío no devuelve nada.
    pub async fn buscar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<CandidatoIngreso>, ErrorConsulta> {
        let criterio = Criterio::desde_texto(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        let encontrados = self
            .fabrica
            .buscar_contratistas(&criterio, limitar(limite))
            .await?;
        let perdidos = self.fabrica.listar_gafetes(TipoGafete::Contratista).await?;
        let mut candidatos = Vec::with_capacity(encontrados.len());
        for contratista in encontrados {
            candidatos.push(self.candidato(contratista, &perdidos).await?);
        }
        Ok(candidatos)
    }

    /// La ficha del contratista elegido.
    pub async fn ficha(&self, id: ContratistaId) -> Result<CandidatoIngreso, ErrorConsulta> {
        let contratista = self
            .fabrica
            .nueva()
            .contratistas()
            .obtener(id)
            .await?
            .ok_or(ErrorCaso::NoEncontrado)?;
        let perdidos = self.fabrica.listar_gafetes(TipoGafete::Contratista).await?;
        self.candidato(contratista, &perdidos).await
    }

    async fn candidato(
        &self,
        contratista: Contratista,
        gafetes: &[ResumenGafete],
    ) -> Result<CandidatoIngreso, ErrorConsulta> {
        let mut uow = self.fabrica.nueva();
        let adentro_por = uow
            .presencias()
            .via_adentro(&Identidad::from(contratista.cedula()))
            .await?;
        let empresa = uow
            .empresas()
            .obtener(contratista.empresa())
            .await?
            .map(|empresa| empresa.nombre().clone());
        let portador = Portador::Contratista(contratista.id());
        let gafetes_perdidos = gafetes
            .iter()
            .filter(|resumen| resumen.gafete.perdido_por(&portador))
            .map(|resumen| resumen.gafete.numero())
            .collect();
        Ok(CandidatoIngreso {
            decision: puede_entrar(&contratista, adentro_por, self.reloj.hoy()),
            contratista,
            empresa,
            gafetes_perdidos,
        })
    }
}
