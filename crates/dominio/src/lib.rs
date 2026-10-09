//! Dominio de Limen: todas las reglas de negocio, sin infraestructura.
//!
//! Regla de dependencias (arquitectura hexagonal): este crate no depende de
//! ningún otro crate de Limen ni de bases de datos, red, reloj del sistema o
//! interfaces. Todo lo demás depende de él. Por ser puro compila igual para
//! escritorio, Android y WebAssembly: no hay un crate de "reglas" aparte.
//!
//! Las reglas que necesitan datos (cédula repetida, "está adentro") también
//! viven acá: el caso de uso averigua el dato y se lo entrega al dominio
//! (ver `HechosContratista`), que es quien decide.

pub mod acceso;
pub mod auditoria;
pub mod cedula;
pub mod contratista;
pub mod empresa;
pub mod gafete;
pub mod ingreso_contratista;
pub mod medio;
pub mod movimiento;
pub mod nombre;
pub mod operador;
pub mod praind;
pub mod presencia;
pub mod reloj;
pub mod tipo_ingreso;
