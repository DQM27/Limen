//! Casos de uso de sólo lectura: lo que muestran las pantallas.

use limen_dominio::busqueda::Criterio;
use limen_dominio::contratista::Contratista;
use limen_dominio::empresa::Empresa;
use limen_dominio::empresa_proveedora::EmpresaProveedora;
use limen_dominio::personal_kof::PersonalKof;

use crate::errores::ErrorCaso;
use crate::puertos::{Consultas, PersonaAdentro};

/// Cuántos resultados devuelve un buscador como máximo: una lista más larga
/// no se lee en la pantalla, y el operador afina lo que escribe.
pub const MAXIMO_RESULTADOS: usize = 50;

/// Las consultas no tienen reglas de negocio propias: no fallan por una
/// regla, sólo por una falla técnica.
pub type ErrorConsulta = ErrorCaso<std::convert::Infallible>;

fn limitar(pedido: usize) -> usize {
    pedido.clamp(1, MAXIMO_RESULTADOS)
}

#[derive(Debug)]
pub struct QuienesEstanAdentro<C> {
    consultas: C,
}

impl<C: Consultas> QuienesEstanAdentro<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(&self) -> Result<Vec<PersonaAdentro>, ErrorConsulta> {
        Ok(self.consultas.quienes_estan_adentro().await?)
    }
}

/// Busca contratistas por cédula (el inicio) o por nombre. Un texto vacío
/// no devuelve nada.
#[derive(Debug)]
pub struct BuscarContratistas<C> {
    consultas: C,
}

impl<C: Consultas> BuscarContratistas<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<Contratista>, ErrorConsulta> {
        let criterio = Criterio::desde_texto(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_contratistas(&criterio, limitar(limite))
            .await?)
    }
}

/// Busca personal KOF por código de empleado (el inicio) o por nombre.
#[derive(Debug)]
pub struct BuscarPersonalKof<C> {
    consultas: C,
}

impl<C: Consultas> BuscarPersonalKof<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<PersonalKof>, ErrorConsulta> {
        let criterio = Criterio::desde_texto(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_personal_kof(&criterio, limitar(limite))
            .await?)
    }
}

/// Busca empresas de contratistas por nombre (también si se escriben
/// números: "3M").
#[derive(Debug)]
pub struct BuscarEmpresas<C> {
    consultas: C,
}

impl<C: Consultas> BuscarEmpresas<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<Empresa>, ErrorConsulta> {
        let criterio = Criterio::de_nombre(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_empresas(&criterio, limitar(limite))
            .await?)
    }
}

/// Busca empresas proveedoras por nombre.
#[derive(Debug)]
pub struct BuscarEmpresasProveedoras<C> {
    consultas: C,
}

impl<C: Consultas> BuscarEmpresasProveedoras<C> {
    pub const fn new(consultas: C) -> Self {
        Self { consultas }
    }

    pub async fn ejecutar(
        &self,
        texto: &str,
        limite: usize,
    ) -> Result<Vec<EmpresaProveedora>, ErrorConsulta> {
        let criterio = Criterio::de_nombre(texto);
        if criterio.es_vacio() {
            return Ok(Vec::new());
        }
        Ok(self
            .consultas
            .buscar_empresas_proveedoras(&criterio, limitar(limite))
            .await?)
    }
}
