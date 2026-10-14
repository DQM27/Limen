-- Lo que sólo existe en la base de un EQUIPO (no en la nube): la bandeja de salida.
--
-- Es el patrón «outbox» (en Lattis, `cola_salida`): cada escritura local anota aquí, en la misma
-- transacción que la cambia, que hay algo que mandar a la nube. La subida vacía la bandeja. Lo que
-- BAJA de la nube se escribe marcado con el parámetro `$de_la_nube` y no pasa por la bandeja, así
-- que nunca rebota. Si otro código escribiera alguna vez en estas tablas sin marcarlo, su cambio
-- sube: lo seguro es subir de más (la nube recibe lo mismo que ya tiene), nunca perder un cambio.
--
-- Una fila por registro (`salida:⟨tabla:clave⟩`), que se reescribe con cada cambio: sólo importa
-- el último estado. `version` sube con cada cambio: al confirmar el envío se quita la fila sólo si
-- la versión sigue siendo la que se mandó, así un cambio hecho mientras se subía no se pierde.
DEFINE TABLE OVERWRITE salida SCHEMAFULL;
DEFINE FIELD OVERWRITE tabla ON salida TYPE string;
DEFINE FIELD OVERWRITE registro ON salida TYPE record;
DEFINE FIELD OVERWRITE borrado ON salida TYPE bool;
DEFINE FIELD OVERWRITE version ON salida TYPE int;
DEFINE FIELD OVERWRITE en ON salida TYPE datetime;
DEFINE INDEX OVERWRITE salida_por_tabla ON salida FIELDS tabla;

-- Un evento por cada tabla que se sincroniza con la nube (la misma lista que
-- `TABLAS_QUE_SUBEN`, que una prueba vigila).
DEFINE EVENT OVERWRITE salir ON TABLE empresa WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'empresa',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE empresa_proveedora WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'empresa_proveedora',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE contratista WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'contratista',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE personal_kof WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'personal_kof',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE gafete WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'gafete',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE ingreso_contratista WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'ingreso_contratista',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE ingreso_proveedor WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'ingreso_proveedor',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE ingreso_correo WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'ingreso_correo',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE prestamo_kof WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'prestamo_kof',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE prestamo_gafete WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'prestamo_gafete',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE presencia WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'presencia',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE personal_kof_con_prestamo WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'personal_kof_con_prestamo',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE hecho WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'hecho',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
DEFINE EVENT OVERWRITE salir ON TABLE auditoria WHEN $de_la_nube != true THEN {
    UPSERT type::record('salida', <string>($after.id ?? $before.id)) SET tabla = 'auditoria',
        registro = ($after.id ?? $before.id), borrado = ($event = 'DELETE'),
        version = (version ?? 0) + 1, en = time::now();
};
