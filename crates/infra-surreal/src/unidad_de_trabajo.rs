//! Unit of Work sobre `SurrealDB`: las escrituras anotadas se confirman en
//! una sola consulta `BEGIN TRANSACTION; … COMMIT TRANSACTION;`, con todos
//! los valores como parámetros enlazados. Si una sentencia falla (por
//! ejemplo, un índice único), la base descarta la transacción entera.

use limen_aplicacion::puertos::{ErrorPersistencia, FabricaUnidadDeTrabajo, UnidadDeTrabajo};

use crate::almacen::AlmacenSurreal;
use crate::error::{al_confirmar, tecnica};
use crate::repositorios::{
    AuditoriaSurreal, ContratistasSurreal, EmpresasSurreal, PresenciasSurreal,
};

impl FabricaUnidadDeTrabajo for AlmacenSurreal {
    type Uow = UowSurreal;

    fn nueva(&self) -> UowSurreal {
        UowSurreal {
            almacen: self.clone(),
            contratistas: ContratistasSurreal::new(self.db().clone()),
            empresas: EmpresasSurreal::new(self.db().clone()),
            presencias: PresenciasSurreal::new(self.db().clone()),
            auditoria: AuditoriaSurreal::default(),
        }
    }
}

#[derive(Debug)]
pub struct UowSurreal {
    almacen: AlmacenSurreal,
    contratistas: ContratistasSurreal,
    empresas: EmpresasSurreal,
    presencias: PresenciasSurreal,
    auditoria: AuditoriaSurreal,
}

impl UnidadDeTrabajo for UowSurreal {
    type Contratistas = ContratistasSurreal;
    type Empresas = EmpresasSurreal;
    type Presencias = PresenciasSurreal;
    type Auditoria = AuditoriaSurreal;

    fn contratistas(&mut self) -> &mut ContratistasSurreal {
        &mut self.contratistas
    }

    fn empresas(&mut self) -> &mut EmpresasSurreal {
        &mut self.empresas
    }

    fn presencias(&self) -> &PresenciasSurreal {
        &self.presencias
    }

    fn auditoria(&mut self) -> &mut AuditoriaSurreal {
        &mut self.auditoria
    }

    async fn confirmar(self) -> Result<(), ErrorPersistencia> {
        let empresas = self.empresas.pendientes;
        let contratistas = self.contratistas.pendientes;
        let auditoria = self.auditoria.pendientes;
        if empresas.is_empty() && contratistas.is_empty() && auditoria.is_empty() {
            return Ok(());
        }

        // Las empresas van primero: un contratista nuevo puede apuntar a una
        // empresa creada en la misma transacción.
        let mut sentencias = vec!["BEGIN TRANSACTION;".to_owned()];
        sentencias.extend(
            (0..empresas.len()).map(|i| format!("UPSERT $empresa_id_{i} CONTENT $empresa_{i};")),
        );
        sentencias.extend(
            (0..contratistas.len())
                .map(|i| format!("UPSERT $contratista_id_{i} CONTENT $contratista_{i};")),
        );
        sentencias.extend(
            (0..auditoria.len())
                .map(|i| format!("CREATE $auditoria_id_{i} CONTENT $auditoria_{i};")),
        );
        sentencias.push("COMMIT TRANSACTION;".to_owned());

        let mut consulta = self.almacen.db().query(sentencias.join("\n"));
        for (i, (id, datos)) in empresas.into_iter().enumerate() {
            consulta = consulta
                .bind((format!("empresa_id_{i}"), id))
                .bind((format!("empresa_{i}"), datos));
        }
        for (i, (id, datos)) in contratistas.into_iter().enumerate() {
            consulta = consulta
                .bind((format!("contratista_id_{i}"), id))
                .bind((format!("contratista_{i}"), datos));
        }
        for (i, (id, entrada)) in auditoria.into_iter().enumerate() {
            consulta = consulta
                .bind((format!("auditoria_id_{i}"), id))
                .bind((format!("auditoria_{i}"), entrada));
        }

        let mut respuesta = consulta.await.map_err(tecnica)?;
        let errores = respuesta.take_errors();
        if errores.is_empty() {
            return Ok(());
        }
        // En una transacción fallida todas las sentencias informan error,
        // pero sólo una dice la causa: se busca la que es un conflicto.
        let traducidos: Vec<ErrorPersistencia> = errores.values().map(al_confirmar).collect();
        Err(traducidos
            .iter()
            .find(|error| matches!(error, ErrorPersistencia::Conflicto(_)))
            .cloned()
            .unwrap_or_else(|| {
                let detalle: Vec<String> = traducidos.iter().map(ToString::to_string).collect();
                ErrorPersistencia::Tecnica(detalle.join("; "))
            }))
    }
}
