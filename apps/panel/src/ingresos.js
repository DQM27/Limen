// Ingresos de todas las unidades: lo que está pasando (quién está adentro ahora) y lo que ya
// pasó (el historial), por las cuatro vías. Aquí sólo se orquesta: leer las cuatro tablas y
// juntarlas. **Lo que se decide lo decide el núcleo** (`dominio.js`): los accesos rápidos de
// fecha (hoy, semana, mes…), qué instantes abarca un rango, el orden en que se muestra y el tope
// de movimientos.

import {
  atajos_de_fecha,
  instantes_del_rango,
  maximo_de_movimientos,
  orden_de_movimientos,
} from './dominio.js';
import { aplicar, texto } from './escritura.js';
import { consultar, resultado, LECTURA } from './nube.js';

export const ESTADOS = ['adentro', 'historial'];
export const VIAS = ['CONTRATISTA', 'PROVEEDOR', 'CORREO', 'KOF'];

/**
 * Cómo se lee cada vía. Todas devuelven la misma forma; `entrada` y `salida` son los campos de
 * su tabla (el personal KOF entra al recibir el gafete provisional y sale al devolverlo).
 */
const TABLAS = {
  CONTRATISTA: {
    tabla: 'ingreso_contratista',
    entrada: 'entrada_en',
    salida: 'salida_en',
    campos: `cedula AS identidad, contratista.nombre AS nombre,
      contratista.empresa.nombre AS procedencia, placa, gafete,
      entrega_gafete = 'SIN_GAFETE' AS sin_gafete, entrada_en AS entrada,
      entrada_operador AS entrada_por, salida_en AS salida, salida_operador AS salida_por`,
  },
  PROVEEDOR: {
    tabla: 'ingreso_proveedor',
    entrada: 'entrada_en',
    salida: 'salida_en',
    campos: `cedula AS identidad, nombre, empresa.nombre AS procedencia, placa, gafete,
      false AS sin_gafete, entrada_en AS entrada, entrada_operador AS entrada_por,
      salida_en AS salida, salida_operador AS salida_por`,
  },
  CORREO: {
    tabla: 'ingreso_correo',
    entrada: 'entrada_en',
    salida: 'salida_en',
    campos: `cedula AS identidad, nombre, motivo AS procedencia, placa, gafete,
      false AS sin_gafete, entrada_en AS entrada, entrada_operador AS entrada_por,
      salida_en AS salida, salida_operador AS salida_por`,
  },
  KOF: {
    tabla: 'prestamo_kof',
    entrada: 'entrega_en',
    salida: 'devolucion_en',
    campos: `codigo_empleado AS identidad, nombre, 'Personal KOF' AS procedencia,
      NONE AS placa, gafete, false AS sin_gafete, entrega_en AS entrada,
      entrega_operador AS entrada_por, devolucion_en AS salida,
      devolucion_operador AS salida_por`,
  },
};

/** Los accesos rápidos de fecha, tal como los da el núcleo para hoy. */
export function atajos() {
  return { estado: 200, cuerpo: JSON.parse(atajos_de_fecha(Date.now())) };
}

/** La consulta de una vía con los filtros pedidos. Los datos van como parámetros. */
function consultaDe(via, { estado, desde, hasta, unidad }, limite) {
  const t = TABLAS[via];
  const condiciones = [];
  if (estado === 'adentro') condiciones.push(`${t.salida} IS NONE`);
  if (desde) condiciones.push(`${t.entrada} >= <datetime>$desde`);
  if (hasta) condiciones.push(`${t.entrada} < <datetime>$hasta`);
  if (unidad) condiciones.push('unidad = $unidad');
  const donde = condiciones.length > 0 ? `WHERE ${condiciones.join(' AND ')}` : '';
  return (
    `SELECT <string>record::id(id) AS id, unidad, '${via}' AS via, ${t.campos} ` +
    `FROM ${t.tabla} ${donde} ORDER BY ${t.entrada} DESC LIMIT ${limite};`
  );
}

/** Quién registró cada movimiento: los usuarios que conoce la nube, por su identificador. */
async function nombresDeOperadores(env) {
  const [s] = await consultar(
    env,
    LECTURA,
    'SELECT <string>record::id(id) AS id, nombre FROM usuario;',
  );
  return new Map(resultado(s).map((u) => [String(u.id).toLowerCase(), u.nombre]));
}

const operador = (nombres, id) => (id ? (nombres.get(String(id).toLowerCase()) ?? null) : null);

/**
 * Los movimientos que pide la pantalla. `estado`: `adentro` (los que todavía no han salido, sin
 * importar la fecha) o `historial` (los que entraron en el rango de días). `via` y `unidad`
 * acotan; vacíos, trae todo.
 */
export function listar(env, parametros) {
  return aplicar(async () => {
    const estado = ESTADOS.includes(parametros.estado) ? parametros.estado : 'adentro';
    const via = texto(parametros.via);
    const unidad = texto(parametros.unidad).slice(0, 60);
    const vias = VIAS.includes(via) ? [via] : VIAS;

    let desde = null;
    let hasta = null;
    let rango = { desde: '', hasta: '' };
    if (estado === 'historial') {
      rango = { desde: texto(parametros.desde), hasta: texto(parametros.hasta) };
      const instantes = JSON.parse(instantes_del_rango(rango.desde, rango.hasta));
      if (instantes.problema) return { estado: 422, cuerpo: { problemas: [instantes.problema] } };
      ({ desde, hasta } = instantes);
    }

    const tope = maximo_de_movimientos() + 1;
    const filtros = { estado, desde, hasta, unidad };
    const variables = {};
    if (desde) variables.desde = desde;
    if (hasta) variables.hasta = hasta;
    if (unidad) variables.unidad = unidad;
    const [respuestas, nombres] = await Promise.all([
      consultar(env, LECTURA, vias.map((v) => consultaDe(v, filtros, tope)).join(' '), variables),
      nombresDeOperadores(env),
    ]);

    const juntos = respuestas.flatMap((r) => resultado(r));
    // El orden y el tope los pone el núcleo.
    const orden = JSON.parse(
      orden_de_movimientos(
        JSON.stringify(juntos.map((m) => ({ entrada: m.entrada, identidad: m.identidad }))),
      ),
    );
    const truncado = juntos.length > maximo_de_movimientos();
    const ingresos = orden
      .slice(0, maximo_de_movimientos())
      .map((i) => juntos[i])
      .map((m) => ({
        id: m.id,
        via: m.via,
        unidad: m.unidad,
        identidad: m.identidad,
        nombre: m.nombre,
        procedencia: m.procedencia ?? '',
        placa: m.placa ?? null,
        gafete: m.gafete ?? null,
        sin_gafete: m.sin_gafete === true,
        entrada: m.entrada,
        entrada_por: operador(nombres, m.entrada_por),
        salida: m.salida ?? null,
        salida_por: operador(nombres, m.salida_por),
      }));
    return { estado: 200, cuerpo: { ingresos, truncado, rango } };
  });
}
