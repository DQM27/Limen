// Ingresos del panel: el Worker junta las cuatro vías; fechas, orden y tope los pone el núcleo (el
// WebAssembly de verdad). La base se simula con un `fetch` falso que anota lo que se le pide.
// Se corre con: node --import ./apps/panel/test/wasm.mjs --test apps/panel/test/ingresos.test.mjs
import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';
import { atajos, listar } from '../src/ingresos.js';

const ENV = { SURREAL_URL: 'https://nube.prueba', SURREAL_USUARIO: 'lector', SURREAL_CLAVE: 'x' };
const ANA = '0199c7a2-0000-7000-8000-000000000001';

const original = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = original;
});

const ok = (result) => ({ status: 'OK', result });

function fila(via, identidad, entrada, extra = {}) {
  return {
    id: `id-${identidad}`,
    via,
    unidad: 'BRISAS',
    identidad,
    nombre: `NOMBRE ${identidad}`,
    procedencia: 'ACME',
    placa: null,
    gafete: 1,
    sin_gafete: false,
    entrada,
    entrada_por: ANA,
    salida: null,
    salida_por: null,
    ...extra,
  };
}

/** Una base de mentira: `porVia` dice qué devuelve cada tabla; los usuarios son sólo Ana. */
function baseFalsa(porVia = {}) {
  const llamadas = [];
  globalThis.fetch = async (url, opciones) => {
    if (String(url).endsWith('/signin')) return Response.json({ token: 'token-de-prueba' });
    const [texto, variables] = JSON.parse(opciones.body).params;
    llamadas.push({ texto, variables });
    if (texto.includes('FROM usuario'))
      return Response.json({ result: [ok([{ id: ANA, nombre: 'ANA MORA' }])] });
    const tablas = [
      ['ingreso_contratista', 'CONTRATISTA'],
      ['ingreso_proveedor', 'PROVEEDOR'],
      ['ingreso_correo', 'CORREO'],
      ['prestamo_kof', 'KOF'],
    ].filter(([tabla]) => texto.includes(`FROM ${tabla} `));
    return Response.json({ result: tablas.map(([, via]) => ok(porVia[via] ?? [])) });
  };
  return llamadas;
}

const consultaDeIngresos = (llamadas) => llamadas.find((l) => !l.texto.includes('FROM usuario'));

test('adentro: trae lo que no ha salido, de todas las vías, sin mirar fechas', async () => {
  const llamadas = baseFalsa({
    CONTRATISTA: [fila('CONTRATISTA', '111', '2026-10-14T10:00:00Z')],
    KOF: [fila('KOF', '222', '2026-10-14T12:00:00Z')],
  });
  const { estado, cuerpo } = await listar(ENV, { estado: 'adentro' });

  assert.equal(estado, 200);
  assert.deepEqual(
    cuerpo.ingresos.map((i) => i.identidad),
    ['222', '111'],
    'lo más reciente primero, como lo ordena el núcleo',
  );
  const { texto, variables } = consultaDeIngresos(llamadas);
  for (const campo of ['salida_en IS NONE', 'devolucion_en IS NONE']) {
    assert.ok(texto.includes(campo), campo);
  }
  assert.ok(!texto.includes('$desde'), 'adentro no mira fechas');
  assert.deepEqual(variables, {});
  assert.equal(cuerpo.ingresos[0].entrada_por, 'ANA MORA', 'el nombre de quien lo registró');
});

test('historial: el rango de días se vuelve instantes de Costa Rica, con el día final completo', async () => {
  const llamadas = baseFalsa();
  const { estado } = await listar(ENV, {
    estado: 'historial',
    desde: '2026-10-13',
    hasta: '2026-10-14',
    unidad: 'BRISAS',
  });

  assert.equal(estado, 200);
  const { texto, variables } = consultaDeIngresos(llamadas);
  assert.equal(variables.desde, '2026-10-13T06:00:00+00:00');
  assert.equal(variables.hasta, '2026-10-15T06:00:00+00:00');
  assert.equal(variables.unidad, 'BRISAS');
  assert.ok(texto.includes('entrada_en >= <datetime>$desde'));
  assert.ok(texto.includes('entrada_en < <datetime>$hasta'));
  assert.ok(texto.includes('entrega_en >= <datetime>$desde'), 'el KOF entra al recibir el gafete');
  assert.ok(!texto.includes('IS NONE'), 'el historial incluye a quienes ya salieron');
});

test('historial: a igual instante se ordena por cédula; el orden es del núcleo', async () => {
  baseFalsa({
    CONTRATISTA: [fila('CONTRATISTA', '300', '2026-10-14T10:00:00Z')],
    PROVEEDOR: [fila('PROVEEDOR', '100', '2026-10-14T10:00:00Z')],
    CORREO: [fila('CORREO', '200', '2026-10-14T11:00:00Z')],
  });
  const { cuerpo } = await listar(ENV, { estado: 'historial', desde: '', hasta: '' });
  assert.deepEqual(
    cuerpo.ingresos.map((i) => i.identidad),
    ['200', '100', '300'],
  );
  assert.equal(cuerpo.truncado, false);
});

test('una sola vía: sólo se consulta esa tabla', async () => {
  const desconocido = '4c415454-4953-0000-0000-000000000001';
  const llamadas = baseFalsa({
    KOF: [fila('KOF', '5043544', '2026-10-09T16:17:30Z', { entrada_por: desconocido })],
  });
  const { cuerpo } = await listar(ENV, { estado: 'historial', via: 'KOF' });
  const { texto } = consultaDeIngresos(llamadas);
  assert.ok(texto.includes('FROM prestamo_kof'));
  assert.ok(!texto.includes('ingreso_contratista'));
  assert.equal(cuerpo.ingresos.length, 1);
  assert.equal(
    cuerpo.ingresos[0].entrada_por,
    null,
    'un usuario que la nube no conoce queda sin nombre',
  );
});

test('un rango al revés lo rechaza el núcleo y no se consulta nada', async () => {
  const llamadas = baseFalsa();
  const { estado, cuerpo } = await listar(ENV, {
    estado: 'historial',
    desde: '2026-10-14',
    hasta: '2026-10-01',
  });
  assert.equal(estado, 422);
  assert.equal(cuerpo.problemas[0].codigo, 'rango_invertido');
  assert.equal(cuerpo.problemas[0].mensaje, 'La fecha inicial no puede ser posterior a la final');
  assert.equal(llamadas.length, 0);

  assert.deepEqual(await listar(ENV, { estado: 'historial', desde: 'ayer' }), {
    estado: 400,
    cuerpo: { error: 'fecha_invalida' },
  });
});

test('los accesos rápidos de fecha vienen del núcleo', () => {
  const { estado, cuerpo } = atajos();
  assert.equal(estado, 200);
  assert.equal(cuerpo.por_omision, 'ULTIMOS_6_MESES');
  assert.deepEqual(cuerpo.atajos.map((a) => a.codigo).slice(0, 3), ['HOY', 'AYER', 'ESTA_SEMANA']);
  assert.match(cuerpo.atajos[0].desde, /^\d{4}-\d{2}-\d{2}$/);
});
