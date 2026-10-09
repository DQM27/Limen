-- Esquema de Limen. Declarativo: se aplica completo cada vez que arranca la
-- app (OVERWRITE), sin ejecutor de migraciones. Ver docs/arquitectura.md, 8.4.
--
-- Los nombres de los índices únicos y de las tablas con clave natural
-- importan: el adaptador los usa para reconocer un choque y traducirlo al
-- error de negocio (ver `error.rs`).

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

-- Catálogo de gafetes (bloque F). Clave natural: gafete:⟨TIPO-NÚMERO⟩, así
-- el mismo número no puede existir dos veces en un tipo.
DEFINE TABLE OVERWRITE gafete SCHEMAFULL;
DEFINE FIELD OVERWRITE tipo ON gafete TYPE 'CONTRATISTA' | 'VISITA' | 'PROVEEDOR' | 'PROVISIONAL_KOF';
DEFINE FIELD OVERWRITE numero ON gafete TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE estado ON gafete TYPE 'DISPONIBLE' | 'PERDIDO' | 'DE_BAJA';
DEFINE FIELD OVERWRITE deudor_contratista ON gafete TYPE option<record<contratista>>;
DEFINE FIELD OVERWRITE deudor_cedula ON gafete TYPE option<string>;

-- Gafetes prestados ahora. Clave: prestamo_gafete:⟨TIPO-NÚMERO⟩; se crea
-- al prestar y se borra al devolver, así un gafete no se presta dos veces.
DEFINE TABLE OVERWRITE prestamo_gafete SCHEMAFULL;
DEFINE FIELD OVERWRITE desde ON prestamo_gafete TYPE datetime;

-- Quién está adentro (reglas E1 y A7). Clave: presencia:⟨cédula⟩; se crea al
-- entrar y se borra al salir, así una persona no está adentro dos veces.
DEFINE TABLE OVERWRITE presencia SCHEMAFULL;
DEFINE FIELD OVERWRITE via ON presencia TYPE 'CONTRATISTA' | 'PROVEEDOR' | 'CORREO';
DEFINE FIELD OVERWRITE desde ON presencia TYPE datetime;

-- Ingresos de contratistas (bloque E).
DEFINE TABLE OVERWRITE ingreso_contratista SCHEMAFULL;
DEFINE FIELD OVERWRITE contratista ON ingreso_contratista TYPE record<contratista>;
DEFINE FIELD OVERWRITE cedula ON ingreso_contratista TYPE string;
DEFINE FIELD OVERWRITE placa ON ingreso_contratista TYPE option<string>;
DEFINE FIELD OVERWRITE gafete ON ingreso_contratista TYPE option<int>;
DEFINE FIELD OVERWRITE entrada_en ON ingreso_contratista TYPE datetime;
DEFINE FIELD OVERWRITE entrada_operador ON ingreso_contratista TYPE uuid;
DEFINE FIELD OVERWRITE salida_en ON ingreso_contratista TYPE option<datetime>;
DEFINE FIELD OVERWRITE salida_operador ON ingreso_contratista TYPE option<uuid>;
DEFINE INDEX OVERWRITE ingreso_contratista_por_gafete ON ingreso_contratista FIELDS gafete, salida_en;

-- Hora del último movimiento del equipo (regla E5): un solo registro,
-- reloj:ultimo.
DEFINE TABLE OVERWRITE reloj SCHEMAFULL;
DEFINE FIELD OVERWRITE en ON reloj TYPE datetime;

-- Auditoría (regla B11): sólo se agrega, nunca se edita ni se borra.
DEFINE TABLE OVERWRITE auditoria SCHEMAFULL;
DEFINE FIELD OVERWRITE entidad ON auditoria TYPE 'contratista' | 'empresa' | 'gafete';
DEFINE FIELD OVERWRITE registro ON auditoria TYPE string;
DEFINE FIELD OVERWRITE accion ON auditoria TYPE 'alta' | 'edicion';
DEFINE FIELD OVERWRITE cambios ON auditoria TYPE array<object>;
DEFINE FIELD OVERWRITE cambios.*.campo ON auditoria TYPE string;
DEFINE FIELD OVERWRITE cambios.*.antes ON auditoria TYPE string;
DEFINE FIELD OVERWRITE cambios.*.despues ON auditoria TYPE string;
DEFINE FIELD OVERWRITE operador ON auditoria TYPE uuid;
DEFINE FIELD OVERWRITE en ON auditoria TYPE datetime;
DEFINE INDEX OVERWRITE auditoria_por_registro ON auditoria FIELDS entidad, registro;
