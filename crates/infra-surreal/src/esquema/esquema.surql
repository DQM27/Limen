-- Esquema de Limen. Declarativo: se aplica completo cada vez que arranca la
-- app (OVERWRITE), sin ejecutor de migraciones. Ver docs/arquitectura.md, 8.4.
--
-- Los nombres de los índices únicos y de las tablas con clave natural
-- importan: el adaptador los usa para reconocer un choque y traducirlo al
-- error de negocio (ver `error.rs`).

-- Buscador: cada palabra del nombre se indexa con todos sus comienzos
-- ("jose" -> j, jo, jos, jose). Buscar "jos pe" exige todos esos comienzos y
-- así encuentra "JOSE PEÑA" con un índice, sin recorrer la tabla. El texto
-- ya está plegado por el dominio (sin tildes, Ñ como N) en `nombre_busqueda`.
DEFINE ANALYZER OVERWRITE buscador_prefijos
    TOKENIZERS blank, punct
    FILTERS lowercase, edgengram(1, 20);

-- Empresas (regla C1: nombre sin repetir).
DEFINE TABLE OVERWRITE empresa SCHEMAFULL;
DEFINE FIELD OVERWRITE nombre ON empresa TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE nombre_busqueda ON empresa TYPE string;
DEFINE INDEX OVERWRITE empresa_nombre_unico ON empresa FIELDS nombre UNIQUE;
DEFINE INDEX OVERWRITE empresa_busqueda ON empresa FIELDS nombre_busqueda
    FULLTEXT ANALYZER buscador_prefijos BM25;

-- Contratistas (reglas B1-B5: cédula sin repetir, empresa existente).
DEFINE TABLE OVERWRITE contratista SCHEMAFULL;
DEFINE FIELD OVERWRITE cedula ON contratista TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE nombre ON contratista TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE empresa ON contratista TYPE record<empresa>;
DEFINE FIELD OVERWRITE tipo_ingreso ON contratista TYPE 'PRAIND' | 'IN_HOUSE';
DEFINE FIELD OVERWRITE fecha_vencimiento_praind ON contratista TYPE datetime;
DEFINE FIELD OVERWRITE tiene_acceso ON contratista TYPE bool;
DEFINE FIELD OVERWRITE nombre_busqueda ON contratista TYPE string;
DEFINE INDEX OVERWRITE contratista_cedula_unica ON contratista FIELDS cedula UNIQUE;
DEFINE INDEX OVERWRITE contratista_busqueda ON contratista FIELDS nombre_busqueda
    FULLTEXT ANALYZER buscador_prefijos BM25;

-- Catálogo de gafetes (bloque F). Un gafete perdido guarda su último
-- portador (F5) en el campo de su clase: contratista, cédula o personal KOF. Clave natural: gafete:⟨TIPO-NÚMERO⟩, así
-- el mismo número no puede existir dos veces en un tipo.
DEFINE TABLE OVERWRITE gafete SCHEMAFULL;
DEFINE FIELD OVERWRITE tipo ON gafete TYPE 'CONTRATISTA' | 'VISITA' | 'PROVEEDOR' | 'PROVISIONAL_KOF';
DEFINE FIELD OVERWRITE numero ON gafete TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE estado ON gafete TYPE 'DISPONIBLE' | 'PERDIDO' | 'DE_BAJA';
DEFINE FIELD OVERWRITE portador_contratista ON gafete TYPE option<record<contratista>>;
DEFINE FIELD OVERWRITE portador_cedula ON gafete TYPE option<string>;
DEFINE FIELD OVERWRITE portador_personal_kof ON gafete TYPE option<record<personal_kof>>;

-- Gafetes prestados ahora. Clave: prestamo_gafete:⟨TIPO-NÚMERO⟩; se crea
-- al prestar y se borra al devolver, así un gafete no se presta dos veces.
DEFINE TABLE OVERWRITE prestamo_gafete SCHEMAFULL;
DEFINE FIELD OVERWRITE desde ON prestamo_gafete TYPE datetime;

-- Quién está adentro (reglas E1 y A7). Clave: presencia:⟨cédula⟩ o, para el
-- personal KOF, presencia:⟨KOF-código de empleado⟩; se crea al entrar y se
-- borra al salir, así una persona no está adentro dos veces.
DEFINE TABLE OVERWRITE presencia SCHEMAFULL;
DEFINE FIELD OVERWRITE via ON presencia TYPE 'CONTRATISTA' | 'PROVEEDOR' | 'CORREO' | 'KOF';
DEFINE FIELD OVERWRITE desde ON presencia TYPE datetime;

-- Ingresos de contratistas (bloque E).
DEFINE TABLE OVERWRITE ingreso_contratista SCHEMAFULL;
DEFINE FIELD OVERWRITE contratista ON ingreso_contratista TYPE record<contratista>;
DEFINE FIELD OVERWRITE cedula ON ingreso_contratista TYPE string;
DEFINE FIELD OVERWRITE placa ON ingreso_contratista TYPE option<string>;
DEFINE FIELD OVERWRITE gafete ON ingreso_contratista TYPE option<int>;
-- Qué pasó con el gafete (E3): prestado (con su número), sin gafete (S/G,
-- decidido por el operador) o no aplica (IN HOUSE).
DEFINE FIELD OVERWRITE entrega_gafete ON ingreso_contratista TYPE option<'PRESTADO' | 'SIN_GAFETE' | 'NO_APLICA'>;
DEFINE FIELD OVERWRITE entrada_en ON ingreso_contratista TYPE datetime;
DEFINE FIELD OVERWRITE entrada_operador ON ingreso_contratista TYPE uuid;
DEFINE FIELD OVERWRITE salida_en ON ingreso_contratista TYPE option<datetime>;
DEFINE FIELD OVERWRITE salida_operador ON ingreso_contratista TYPE option<uuid>;
DEFINE INDEX OVERWRITE ingreso_contratista_por_gafete ON ingreso_contratista FIELDS gafete, salida_en;

-- Empresas proveedoras (regla H1): catálogo aparte del de contratistas.
DEFINE TABLE OVERWRITE empresa_proveedora SCHEMAFULL;
DEFINE FIELD OVERWRITE nombre ON empresa_proveedora TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE nombre_busqueda ON empresa_proveedora TYPE string;
DEFINE INDEX OVERWRITE empresa_proveedora_nombre_unico ON empresa_proveedora FIELDS nombre UNIQUE;
DEFINE INDEX OVERWRITE empresa_proveedora_busqueda ON empresa_proveedora FIELDS nombre_busqueda
    FULLTEXT ANALYZER buscador_prefijos BM25;

-- Ingresos de proveedores (bloque H). Cédula y nombre llegan con cada
-- ingreso: no hay catálogo de personas.
DEFINE TABLE OVERWRITE ingreso_proveedor SCHEMAFULL;
DEFINE FIELD OVERWRITE cedula ON ingreso_proveedor TYPE string;
DEFINE FIELD OVERWRITE nombre ON ingreso_proveedor TYPE string;
DEFINE FIELD OVERWRITE empresa ON ingreso_proveedor TYPE record<empresa_proveedora>;
DEFINE FIELD OVERWRITE placa ON ingreso_proveedor TYPE option<string>;
DEFINE FIELD OVERWRITE gafete ON ingreso_proveedor TYPE int;
DEFINE FIELD OVERWRITE entrada_en ON ingreso_proveedor TYPE datetime;
DEFINE FIELD OVERWRITE entrada_operador ON ingreso_proveedor TYPE uuid;
DEFINE FIELD OVERWRITE salida_en ON ingreso_proveedor TYPE option<datetime>;
DEFINE FIELD OVERWRITE salida_operador ON ingreso_proveedor TYPE option<uuid>;
DEFINE INDEX OVERWRITE ingreso_proveedor_por_gafete ON ingreso_proveedor FIELDS gafete, salida_en;

-- Ingresos por correo (bloque I): visita autorizada por correo. Como el
-- proveedor, pero con motivo en lugar de empresa y gafete de visita.
DEFINE TABLE OVERWRITE ingreso_correo SCHEMAFULL;
DEFINE FIELD OVERWRITE cedula ON ingreso_correo TYPE string;
DEFINE FIELD OVERWRITE nombre ON ingreso_correo TYPE string;
DEFINE FIELD OVERWRITE motivo ON ingreso_correo TYPE string;
DEFINE FIELD OVERWRITE placa ON ingreso_correo TYPE option<string>;
DEFINE FIELD OVERWRITE gafete ON ingreso_correo TYPE int;
DEFINE FIELD OVERWRITE entrada_en ON ingreso_correo TYPE datetime;
DEFINE FIELD OVERWRITE entrada_operador ON ingreso_correo TYPE uuid;
DEFINE FIELD OVERWRITE salida_en ON ingreso_correo TYPE option<datetime>;
DEFINE FIELD OVERWRITE salida_operador ON ingreso_correo TYPE option<uuid>;
DEFINE INDEX OVERWRITE ingreso_correo_por_gafete ON ingreso_correo FIELDS gafete, salida_en;

-- Personal KOF (bloque K): se identifica por código de empleado.
DEFINE TABLE OVERWRITE personal_kof SCHEMAFULL;
DEFINE FIELD OVERWRITE codigo_empleado ON personal_kof TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE nombre ON personal_kof TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE activo ON personal_kof TYPE bool;
DEFINE FIELD OVERWRITE nombre_busqueda ON personal_kof TYPE string;
DEFINE INDEX OVERWRITE personal_kof_codigo_unico ON personal_kof FIELDS codigo_empleado UNIQUE;
DEFINE INDEX OVERWRITE personal_kof_busqueda ON personal_kof FIELDS nombre_busqueda
    FULLTEXT ANALYZER buscador_prefijos BM25;

-- Préstamos del gafete provisional KOF.
DEFINE TABLE OVERWRITE prestamo_kof SCHEMAFULL;
DEFINE FIELD OVERWRITE personal ON prestamo_kof TYPE record<personal_kof>;
DEFINE FIELD OVERWRITE codigo_empleado ON prestamo_kof TYPE string;
DEFINE FIELD OVERWRITE nombre ON prestamo_kof TYPE string;
DEFINE FIELD OVERWRITE gafete ON prestamo_kof TYPE int;
DEFINE FIELD OVERWRITE entrega_en ON prestamo_kof TYPE datetime;
DEFINE FIELD OVERWRITE entrega_operador ON prestamo_kof TYPE uuid;
DEFINE FIELD OVERWRITE devolucion_en ON prestamo_kof TYPE option<datetime>;
DEFINE FIELD OVERWRITE devolucion_operador ON prestamo_kof TYPE option<uuid>;
DEFINE INDEX OVERWRITE prestamo_kof_por_gafete ON prestamo_kof FIELDS gafete, devolucion_en;

-- Personas con un provisional sin devolver. Clave:
-- personal_kof_con_prestamo:⟨uuid de la persona⟩; se crea al entregar y se
-- borra al devolver, así nadie tiene dos provisionales a la vez.
DEFINE TABLE OVERWRITE personal_kof_con_prestamo SCHEMAFULL;
DEFINE FIELD OVERWRITE desde ON personal_kof_con_prestamo TYPE datetime;

-- Hora del último movimiento del equipo (regla E5): un solo registro,
-- reloj:ultimo.
DEFINE TABLE OVERWRITE reloj SCHEMAFULL;
DEFINE FIELD OVERWRITE en ON reloj TYPE datetime;

-- Usuarios (bloque L). La contraseña nunca se guarda: sólo su hash
-- Argon2id en formato PHC (lleva la sal y los parámetros).
DEFINE TABLE OVERWRITE usuario SCHEMAFULL;
DEFINE FIELD OVERWRITE cedula ON usuario TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE nombre ON usuario TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE activo ON usuario TYPE bool;
DEFINE FIELD OVERWRITE contrasena ON usuario TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE debe_cambiar_contrasena ON usuario TYPE bool;
DEFINE INDEX OVERWRITE usuario_cedula_unica ON usuario FIELDS cedula UNIQUE;

-- Intentos fallidos de inicio de sesión (regla L6). Clave:
-- intento_inicio:⟨cédula⟩; se borra al entrar. Son del equipo: no se
-- sincronizan.
DEFINE TABLE OVERWRITE intento_inicio SCHEMAFULL;
DEFINE FIELD OVERWRITE cantidad ON intento_inicio TYPE int;
DEFINE FIELD OVERWRITE ultimo ON intento_inicio TYPE datetime;

-- Auditoría (regla B11): sólo se agrega, nunca se edita ni se borra. El
-- evento lo hace cumplir la base, también para la conexión embebida (que
-- entra como root y no pasa por PERMISSIONS).
DEFINE TABLE OVERWRITE auditoria SCHEMAFULL;
DEFINE FIELD OVERWRITE entidad ON auditoria TYPE 'contratista' | 'empresa' | 'empresa_proveedora' | 'gafete' | 'personal_kof' | 'usuario';
DEFINE FIELD OVERWRITE registro ON auditoria TYPE string;
DEFINE FIELD OVERWRITE accion ON auditoria TYPE 'alta' | 'edicion';
DEFINE FIELD OVERWRITE cambios ON auditoria TYPE array<object>;
DEFINE FIELD OVERWRITE cambios.*.campo ON auditoria TYPE string;
DEFINE FIELD OVERWRITE cambios.*.antes ON auditoria TYPE string;
DEFINE FIELD OVERWRITE cambios.*.despues ON auditoria TYPE string;
DEFINE FIELD OVERWRITE operador ON auditoria TYPE uuid;
DEFINE FIELD OVERWRITE en ON auditoria TYPE datetime;
DEFINE INDEX OVERWRITE auditoria_por_registro ON auditoria FIELDS entidad, registro;
DEFINE EVENT OVERWRITE auditoria_inmutable ON auditoria WHEN $event != "CREATE" THEN {
    THROW "la auditoría no se edita ni se borra";
};

-- Hechos: cada entrada y salida, por las cuatro vías, tal como ocurrió
-- (ver docs/arquitectura.md, sección 9). Los ingresos, la presencia y los
-- préstamos son el estado que se deriva de ellos; los hechos son lo que
-- pasó y lo que se enviará a los demás equipos.
--
-- Nunca se editan ni se borran: los campos son READONLY y el evento
-- rechaza cualquier UPDATE, UPSERT o DELETE. Lo que quedó mal se corrige
-- con otro hecho. La clave es el UUID v7 del hecho: un reintento con el
-- mismo ID no lo duplica.
--
-- Una entrada lleva el registro que abrió en el campo de su vía (el mismo
-- formato que su tabla), para que otro equipo pueda reconstruirlo; una
-- salida no lleva ninguno.
DEFINE TABLE OVERWRITE hecho SCHEMAFULL;
DEFINE FIELD OVERWRITE tipo ON hecho TYPE 'ENTRADA' | 'SALIDA' READONLY;
DEFINE FIELD OVERWRITE via ON hecho TYPE 'CONTRATISTA' | 'PROVEEDOR' | 'CORREO' | 'KOF' READONLY;
DEFINE FIELD OVERWRITE registro ON hecho TYPE uuid READONLY;
DEFINE FIELD OVERWRITE en ON hecho TYPE datetime READONLY;
DEFINE FIELD OVERWRITE operador ON hecho TYPE uuid READONLY;
-- E5: si la hora con que se selló era confiable, y la hora cruda del reloj
-- del equipo en ese momento (un salto entre las dos delata un reloj
-- cambiado a mano).
DEFINE FIELD OVERWRITE hora_confiable ON hecho TYPE bool READONLY;
DEFINE FIELD OVERWRITE hora_equipo ON hecho TYPE datetime READONLY;
DEFINE FIELD OVERWRITE ingreso_contratista ON hecho TYPE option<object> FLEXIBLE READONLY;
DEFINE FIELD OVERWRITE ingreso_proveedor ON hecho TYPE option<object> FLEXIBLE READONLY;
DEFINE FIELD OVERWRITE ingreso_correo ON hecho TYPE option<object> FLEXIBLE READONLY;
DEFINE FIELD OVERWRITE prestamo_kof ON hecho TYPE option<object> FLEXIBLE READONLY;
DEFINE INDEX OVERWRITE hecho_por_registro ON hecho FIELDS registro;
DEFINE EVENT OVERWRITE hecho_inmutable ON hecho WHEN $event != "CREATE" THEN {
    THROW "los hechos no se editan ni se borran";
};
