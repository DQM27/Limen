-- Lo que sólo vive en la nube (se aplica DESPUÉS de esquema.surql, siempre juntos: volver a
-- definir una tabla con OVERWRITE le quita los permisos). Los equipos tienen su propio esquema
-- local y no llevan nada de esto. Ver docs/nube.md, sección 13.
--
-- Cada equipo entra con un token del Worker que lleva su unidad (`DEFINE ACCESS equipo`, que NO
-- está aquí porque lleva la clave de firma: se define aparte, ver docs/nube.md). Un usuario de
-- sistema (los de administración y el panel) no pasa por estos permisos.

-- El diario de cambios. SHOW CHANGES de SurrealDB está prohibido para un equipo con acceso
-- limitado, y además guarda una copia de cada fila; este diario guarda sólo qué fila cambió.
-- Lo llena la base con un evento por tabla y cada equipo lee el de su unidad (y el global, con
-- unidad vacía).
DEFINE TABLE OVERWRITE cambio SCHEMAFULL
    PERMISSIONS FOR select WHERE unidad = $token.unidad OR unidad = ''
    FOR create, update, delete NONE;
DEFINE FIELD OVERWRITE unidad ON cambio TYPE string;
DEFINE FIELD OVERWRITE tabla ON cambio TYPE string;
DEFINE FIELD OVERWRITE registro ON cambio TYPE record;
DEFINE FIELD OVERWRITE tipo ON cambio TYPE string;
DEFINE FIELD OVERWRITE en ON cambio TYPE datetime;
DEFINE INDEX OVERWRITE cambio_por_en ON cambio FIELDS en;

-- Permisos. De la unidad: cada equipo ve y escribe sólo lo de su unidad. Catálogos globales:
-- todos los equipos los leen y los agregan o editan, pero no los borran. Los hechos y la
-- auditoría sólo se agregan (además de los eventos que impiden editarlos).
ALTER TABLE gafete PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
ALTER TABLE prestamo_gafete PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
ALTER TABLE presencia PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
ALTER TABLE ingreso_contratista PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
ALTER TABLE ingreso_proveedor PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
ALTER TABLE ingreso_correo PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
ALTER TABLE prestamo_kof PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
ALTER TABLE personal_kof_con_prestamo PERMISSIONS FOR select, create, update, delete WHERE unidad = $token.unidad;
-- El panel entra con su propio acceso (`panel`, sólo lectura, tiempo real; se define aparte por la
-- clave de firma, ver docs/nube.md sección 14): ve los hechos de todas las unidades y no escribe
-- nada. Los permisos «para todos» (FULL) valdrían también para el panel, así que lo que escriben
-- los equipos se acota a `$access = 'equipo'`.
ALTER TABLE hecho PERMISSIONS FOR select WHERE unidad = $token.unidad OR $access = 'panel'
    FOR create WHERE unidad = $token.unidad;
ALTER TABLE empresa PERMISSIONS FOR select FULL FOR create, update WHERE $access = 'equipo' FOR delete NONE;
ALTER TABLE empresa_proveedora PERMISSIONS FOR select FULL FOR create, update WHERE $access = 'equipo' FOR delete NONE;
ALTER TABLE contratista PERMISSIONS FOR select FULL FOR create, update WHERE $access = 'equipo' FOR delete NONE;
ALTER TABLE personal_kof PERMISSIONS FOR select FULL FOR create, update WHERE $access = 'equipo' FOR delete NONE;
ALTER TABLE auditoria PERMISSIONS FOR select FULL FOR create WHERE $access = 'equipo' FOR update, delete NONE;
-- Los usuarios y las unidades los administra la nube: el equipo sólo los lee.
ALTER TABLE usuario PERMISSIONS FOR select FULL FOR create, update, delete NONE;
ALTER TABLE unidad_operativa PERMISSIONS FOR select FULL FOR create, update, delete NONE;
-- La clave (su hash) de un usuario sólo la ve el equipo, que la necesita para iniciar sesión:
-- el panel ve a los usuarios pero nunca sus claves.
DEFINE FIELD OVERWRITE clave ON usuario TYPE string ASSERT string::len($value) > 0
    PERMISSIONS FOR select WHERE $access = 'equipo';
-- Lo que sólo lee el panel: qué equipos hay y cuáles están conectados.
ALTER TABLE conexion PERMISSIONS FOR select WHERE $access = 'panel' FOR create, update, delete NONE;
ALTER TABLE dispositivo PERMISSIONS FOR select WHERE $access = 'panel' FOR create, update, delete NONE;

-- Eventos: anotan en el diario cada alta, cambio y borrado, dentro de la misma transacción.
DEFINE EVENT OVERWRITE anotar ON TABLE gafete THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'gafete',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE prestamo_gafete THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'prestamo_gafete',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE presencia THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'presencia',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE ingreso_contratista THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'ingreso_contratista',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE ingreso_proveedor THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'ingreso_proveedor',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE ingreso_correo THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'ingreso_correo',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE prestamo_kof THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'prestamo_kof',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE personal_kof_con_prestamo THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'personal_kof_con_prestamo',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE hecho THEN {
    CREATE cambio SET unidad = ($after.unidad ?? $before.unidad ?? ''), tabla = 'hecho',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE empresa THEN {
    CREATE cambio SET unidad = '', tabla = 'empresa',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE empresa_proveedora THEN {
    CREATE cambio SET unidad = '', tabla = 'empresa_proveedora',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE contratista THEN {
    CREATE cambio SET unidad = '', tabla = 'contratista',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE personal_kof THEN {
    CREATE cambio SET unidad = '', tabla = 'personal_kof',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE auditoria THEN {
    CREATE cambio SET unidad = '', tabla = 'auditoria',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
DEFINE EVENT OVERWRITE anotar ON TABLE usuario THEN {
    CREATE cambio SET unidad = '', tabla = 'usuario',
        registro = ($after.id ?? $before.id), tipo = $event, en = time::now();
};
