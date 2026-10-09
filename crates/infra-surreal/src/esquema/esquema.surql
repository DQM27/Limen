-- Esquema de Limen. Declarativo: se aplica completo cada vez que arranca la
-- app (OVERWRITE), sin ejecutor de migraciones. Ver docs/arquitectura.md, 8.4.
--
-- Los nombres de los índices únicos importan: el adaptador los usa para
-- reconocer un choque y traducirlo al error de negocio (ver `error.rs`).

-- Empresas (regla C1: nombre sin repetir).
DEFINE TABLE OVERWRITE empresa SCHEMAFULL;
DEFINE FIELD OVERWRITE nombre ON empresa TYPE string ASSERT string::len($value) > 0;
DEFINE INDEX OVERWRITE empresa_nombre_unico ON empresa FIELDS nombre UNIQUE;

-- Contratistas (reglas B1-B5: cédula sin repetir, empresa existente).
DEFINE TABLE OVERWRITE contratista SCHEMAFULL;
DEFINE FIELD OVERWRITE cedula ON contratista TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE nombre ON contratista TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE empresa ON contratista TYPE record<empresa>;
DEFINE FIELD OVERWRITE tipo_ingreso ON contratista TYPE 'PRAIND' | 'IN_HOUSE';
DEFINE FIELD OVERWRITE fecha_vencimiento_praind ON contratista TYPE datetime;
DEFINE FIELD OVERWRITE tiene_acceso ON contratista TYPE bool;
DEFINE INDEX OVERWRITE contratista_cedula_unica ON contratista FIELDS cedula UNIQUE;

-- Quién está adentro. La llenará el módulo de ingresos; la clave del
-- registro es el ID del contratista, así que no puede estar dos veces.
DEFINE TABLE OVERWRITE presencia SCHEMAFULL;
DEFINE FIELD OVERWRITE desde ON presencia TYPE datetime;

-- Auditoría (regla B11): sólo se agrega, nunca se edita ni se borra.
DEFINE TABLE OVERWRITE auditoria SCHEMAFULL;
DEFINE FIELD OVERWRITE entidad ON auditoria TYPE 'contratista' | 'empresa';
DEFINE FIELD OVERWRITE registro ON auditoria TYPE uuid;
DEFINE FIELD OVERWRITE accion ON auditoria TYPE 'alta' | 'edicion';
DEFINE FIELD OVERWRITE cambios ON auditoria TYPE array<object>;
DEFINE FIELD OVERWRITE cambios[*].campo ON auditoria TYPE string;
DEFINE FIELD OVERWRITE cambios[*].antes ON auditoria TYPE string;
DEFINE FIELD OVERWRITE cambios[*].despues ON auditoria TYPE string;
DEFINE FIELD OVERWRITE operador ON auditoria TYPE uuid;
DEFINE FIELD OVERWRITE en ON auditoria TYPE datetime;
DEFINE INDEX OVERWRITE auditoria_por_registro ON auditoria FIELDS registro;
