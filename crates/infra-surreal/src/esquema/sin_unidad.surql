-- Convierte lo guardado antes de las unidades operativas (M4) a la forma
-- nueva. Corre al abrir la base, después del esquema, y sólo toca lo que
-- todavía no tiene unidad: en una base nueva o ya convertida no hace nada.
--
-- Todo lo que había en la base de un equipo es de su unidad ($unidad).
-- Los gafetes y sus préstamos cambian de clave (TIPO-NÚMERO pasa a
-- UNIDAD-TIPO-NÚMERO); lo demás sólo recibe el campo `unidad`. Los hechos
-- no se tocan: no se editan nunca. Los que no traen origen se leen como
-- registrados en este equipo, que es donde se registraron.
BEGIN TRANSACTION;

FOR $gafete IN (SELECT * FROM gafete WHERE unidad = NONE) {
    CREATE type::record('gafete', $unidad + '-' + record::id($gafete.id)) CONTENT {
        unidad: $unidad,
        tipo: $gafete.tipo,
        numero: $gafete.numero,
        estado: $gafete.estado,
        portador_contratista: $gafete.portador_contratista,
        portador_cedula: $gafete.portador_cedula,
        portador_personal_kof: $gafete.portador_personal_kof,
    };
    DELETE $gafete.id;
};

FOR $prestamo IN (SELECT * FROM prestamo_gafete WHERE unidad = NONE) {
    CREATE type::record('prestamo_gafete', $unidad + '-' + record::id($prestamo.id)) CONTENT {
        unidad: $unidad,
        desde: $prestamo.desde,
    };
    DELETE $prestamo.id;
};

UPDATE presencia SET unidad = $unidad WHERE unidad = NONE;
UPDATE ingreso_contratista SET unidad = $unidad WHERE unidad = NONE;
UPDATE ingreso_proveedor SET unidad = $unidad WHERE unidad = NONE;
UPDATE ingreso_correo SET unidad = $unidad WHERE unidad = NONE;
UPDATE prestamo_kof SET unidad = $unidad WHERE unidad = NONE;
UPDATE personal_kof_con_prestamo SET unidad = $unidad WHERE unidad = NONE;

COMMIT TRANSACTION;
