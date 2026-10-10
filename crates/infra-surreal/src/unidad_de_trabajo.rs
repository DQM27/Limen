//! Unit of Work sobre `SurrealDB`: las escrituras anotadas se confirman en
//! una sola consulta `BEGIN TRANSACTION; … COMMIT TRANSACTION;`, con todos
//! los valores como parámetros enlazados. Si una sentencia falla (un índice
//! único, una clave natural repetida), la base descarta la transacción
//! entera.

use limen_aplicacion::puertos::{ErrorPersistencia, FabricaUnidadDeTrabajo, UnidadDeTrabajo};

use crate::almacen::AlmacenSurreal;
use crate::error::{al_confirmar, tecnica};
use crate::repositorios::{
    AuditoriaSurreal, ContratistasSurreal, EmpresasProveedorasSurreal, EmpresasSurreal, Escritura,
    GafetesSurreal, HechosSurreal, IngresosCorreoSurreal, IngresosProveedorSurreal,
    IngresosSurreal, PersonalKofSurreal, PresenciasSurreal, PrestamosKofSurreal, RelojSurreal,
};

impl FabricaUnidadDeTrabajo for AlmacenSurreal {
    type Uow = UowSurreal;

    fn nueva(&self) -> UowSurreal {
        let db = self.db();
        UowSurreal {
            almacen: self.clone(),
            contratistas: ContratistasSurreal::new(db.clone()),
            empresas: EmpresasSurreal::new(db.clone()),
            presencias: PresenciasSurreal::new(db.clone()),
            gafetes: GafetesSurreal::new(db.clone()),
            ingresos: IngresosSurreal::new(db.clone()),
            empresas_proveedoras: EmpresasProveedorasSurreal::new(db.clone()),
            ingresos_proveedor: IngresosProveedorSurreal::new(db.clone()),
            ingresos_correo: IngresosCorreoSurreal::new(db.clone()),
            personal_kof: PersonalKofSurreal::new(db.clone()),
            prestamos_kof: PrestamosKofSurreal::new(db.clone()),
            reloj: RelojSurreal::new(db.clone()),
            auditoria: AuditoriaSurreal::default(),
            hechos: HechosSurreal::default(),
        }
    }
}

#[derive(Debug)]
pub struct UowSurreal {
    almacen: AlmacenSurreal,
    contratistas: ContratistasSurreal,
    empresas: EmpresasSurreal,
    presencias: PresenciasSurreal,
    gafetes: GafetesSurreal,
    ingresos: IngresosSurreal,
    empresas_proveedoras: EmpresasProveedorasSurreal,
    ingresos_proveedor: IngresosProveedorSurreal,
    ingresos_correo: IngresosCorreoSurreal,
    personal_kof: PersonalKofSurreal,
    prestamos_kof: PrestamosKofSurreal,
    reloj: RelojSurreal,
    auditoria: AuditoriaSurreal,
    hechos: HechosSurreal,
}

impl UnidadDeTrabajo for UowSurreal {
    type Contratistas = ContratistasSurreal;
    type Empresas = EmpresasSurreal;
    type Presencias = PresenciasSurreal;
    type Gafetes = GafetesSurreal;
    type Ingresos = IngresosSurreal;
    type EmpresasProveedoras = EmpresasProveedorasSurreal;
    type IngresosProveedor = IngresosProveedorSurreal;
    type IngresosCorreo = IngresosCorreoSurreal;
    type PersonalKof = PersonalKofSurreal;
    type PrestamosKof = PrestamosKofSurreal;
    type Reloj = RelojSurreal;
    type Auditoria = AuditoriaSurreal;
    type Hechos = HechosSurreal;

    fn contratistas(&mut self) -> &mut ContratistasSurreal {
        &mut self.contratistas
    }

    fn empresas(&mut self) -> &mut EmpresasSurreal {
        &mut self.empresas
    }

    fn presencias(&mut self) -> &mut PresenciasSurreal {
        &mut self.presencias
    }

    fn gafetes(&mut self) -> &mut GafetesSurreal {
        &mut self.gafetes
    }

    fn ingresos(&mut self) -> &mut IngresosSurreal {
        &mut self.ingresos
    }

    fn empresas_proveedoras(&mut self) -> &mut EmpresasProveedorasSurreal {
        &mut self.empresas_proveedoras
    }

    fn ingresos_proveedor(&mut self) -> &mut IngresosProveedorSurreal {
        &mut self.ingresos_proveedor
    }

    fn ingresos_correo(&mut self) -> &mut IngresosCorreoSurreal {
        &mut self.ingresos_correo
    }

    fn personal_kof(&mut self) -> &mut PersonalKofSurreal {
        &mut self.personal_kof
    }

    fn prestamos_kof(&mut self) -> &mut PrestamosKofSurreal {
        &mut self.prestamos_kof
    }

    fn reloj(&mut self) -> &mut RelojSurreal {
        &mut self.reloj
    }

    fn auditoria(&mut self) -> &mut AuditoriaSurreal {
        &mut self.auditoria
    }

    fn hechos(&mut self) -> &mut HechosSurreal {
        &mut self.hechos
    }

    async fn confirmar(self) -> Result<(), ErrorPersistencia> {
        // El orden importa: las empresas antes que los contratistas (uno
        // nuevo puede apuntar a una empresa creada en la misma transacción),
        // y dentro de cada repositorio, en el orden en que se anotaron.
        let escrituras: Vec<Escritura> = [
            self.empresas.pendientes,
            self.contratistas.pendientes,
            self.empresas_proveedoras.pendientes,
            self.gafetes.pendientes,
            self.presencias.pendientes,
            self.ingresos.pendientes,
            self.ingresos_proveedor.pendientes,
            self.ingresos_correo.pendientes,
            self.personal_kof.pendientes,
            self.prestamos_kof.pendientes,
            self.reloj.pendientes,
            self.auditoria.pendientes,
            self.hechos.pendientes,
        ]
        .into_iter()
        .flatten()
        .collect();
        if escrituras.is_empty() {
            return Ok(());
        }

        let mut sentencias = vec!["BEGIN TRANSACTION;".to_owned()];
        sentencias.extend(
            escrituras
                .iter()
                .enumerate()
                .map(|(i, escritura)| match escritura {
                    Escritura::Crear(..) => format!("CREATE $id_{i} CONTENT $datos_{i};"),
                    Escritura::Guardar(..) => format!("UPSERT $id_{i} CONTENT $datos_{i};"),
                    Escritura::Borrar(_) => format!("DELETE $id_{i};"),
                }),
        );
        sentencias.push("COMMIT TRANSACTION;".to_owned());

        let mut consulta = self.almacen.db().query(sentencias.join("\n"));
        for (i, escritura) in escrituras.into_iter().enumerate() {
            consulta = match escritura {
                Escritura::Crear(id, datos) | Escritura::Guardar(id, datos) => consulta
                    .bind((format!("id_{i}"), id))
                    .bind((format!("datos_{i}"), datos)),
                Escritura::Borrar(id) => consulta.bind((format!("id_{i}"), id)),
            };
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
