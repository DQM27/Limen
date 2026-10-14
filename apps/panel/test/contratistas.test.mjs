// Contratistas del panel: el Worker sólo orquesta; lo que se decide lo decide el núcleo (el
// WebAssembly de verdad). La base se simula con un `fetch` falso que anota lo que se le pide.
// Se corre con: node --import ./apps/panel/test/wasm.mjs --test apps/panel/test/contratistas.test.mjs
import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';
import { crearEmpresa, editar, listar, registrar } from '../src/contratistas.js';
import { ADMINISTRADOR_DEL_PANEL } from '../src/escritura.js';

const ID = '0199c7a2-0000-7000-8000-000000000001';
const EMPRESA = '0199c7a2-0000-7000-8000-0000000000aa';
const ENV = {
  SURREAL_URL: 'https://nube.prueba',
  SURREAL_USUARIO: 'lector',
  SURREAL_CLAVE: 'x',
  SURREAL_ADMIN_USUARIO: 'admin',
  SURREAL_ADMIN_CLAVE: 'y',
};
const JOSE = {
  id: ID,
  cedula: '111111111',
  nombre: 'JOSE PEÑA',
  empresa_id: EMPRESA,
  empresa_nombre: 'ACME',
  tipo_ingreso: 'PRAIND',
  fecha_vencimiento_praind: '2999-01-01',
  tiene_acceso: true,
};
const FORMULARIO = {
  cedula: '1-1111-1111',
  nombre: '  josé  peña ',
  empresa_id: EMPRESA,
  tipo_ingreso: 'PRAIND',
  fecha_vencimiento_praind: '2999-01-01',
  tiene_acceso: true,
};

const original = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = original;
});

const ok = (result) => ({ status: 'OK', result });

function baseFalsa(responder) {
  const llamadas = [];
  globalThis.fetch = async (url, opciones) => {
    if (String(url).endsWith('/signin')) return Response.json({ token: 'token-de-prueba' });
    const [texto, variables] = JSON.parse(opciones.body).params;
    llamadas.push({ texto, variables });
    return Response.json({ result: responder(texto, variables) });
  };
  return llamadas;
}

const escrituras = (llamadas) => llamadas.filter((l) => l.texto.startsWith('BEGIN'));

/** Una base donde la empresa existe, y `cedulaDe` (si se da) ya tiene la cédula. */
const base =
  ({ cedulaDe = [], empresa = [ok([EMPRESA])], adentro = [], jose = [ok([JOSE])] } = {}) =>
  (texto) => {
    if (texto.includes('FROM contratista WHERE cedula')) return [ok(cedulaDe)];
    if (texto.startsWith('SELECT VALUE nombre FROM empresa')) return [ok(['ACME'])];
    if (texto.includes('FROM empresa WHERE')) return empresa;
    if (texto.includes('FROM presencia')) return [ok(adentro)];
    if (texto.includes('FROM contratista WHERE') && texto.includes('LIMIT 1')) return jose;
    if (texto.startsWith('SELECT VALUE nombre FROM empresa')) return [ok(['ACME'])];
    return [ok([]), ok([])];
  };

test('registrar: el núcleo normaliza, se guarda con su búsqueda y queda auditado', async () => {
  const llamadas = baseFalsa(base());
  const { estado, cuerpo } = await registrar(ENV, FORMULARIO);

  assert.equal(estado, 201);
  assert.equal(cuerpo.contratista.cedula, '111111111');
  assert.equal(cuerpo.contratista.nombre, 'JOSE PEÑA');
  assert.equal(cuerpo.contratista.empresa_nombre, 'ACME');

  const [escritura] = escrituras(llamadas);
  assert.match(escritura.texto, /CREATE type::record\('contratista'/);
  assert.equal(escritura.variables.nombre_busqueda, 'JOSE PENA');
  assert.equal(escritura.variables.fecha, '2999-01-01T00:00:00Z');
  assert.equal(escritura.variables.accion, 'alta');
  assert.equal(escritura.variables.entidad, 'contratista');
  assert.equal(escritura.variables.operador, ADMINISTRADOR_DEL_PANEL);
  assert.equal(escritura.variables.cambios.length, 6);
  // La base lee como UUID el texto que lo parezca (la empresa); la auditoría guarda texto.
  assert.match(
    escritura.texto,
    /<string>\$c\.despues/,
    'la auditoría devuelve los valores a texto',
  );
});

test('registrar: lo que no vale vuelve completo, campo por campo, y no se escribe nada', async () => {
  const llamadas = baseFalsa(base({ empresa: [ok([])] }));
  const { estado, cuerpo } = await registrar(ENV, {
    cedula: 'AB1',
    nombre: '',
    empresa_id: '',
    tipo_ingreso: 'PRAIND',
    fecha_vencimiento_praind: '',
  });

  assert.equal(estado, 422);
  assert.deepEqual(
    cuerpo.problemas.map((p) => p.campo),
    ['cedula', 'nombre', 'empresa_id', 'fecha_vencimiento_praind'],
  );
  assert.equal(
    cuerpo.problemas[0].mensaje,
    'La cédula debe tener sólo números, entre 9 y 13 dígitos',
  );
  assert.equal(escrituras(llamadas).length, 0);
});

test('registrar: la cédula de otro contratista y un PRAIND vencido los dice el núcleo', async () => {
  baseFalsa(base({ cedulaDe: [ok(['0199c7a2-0000-7000-8000-0000000000cc'])] }));
  const repetida = await registrar(ENV, FORMULARIO);
  assert.equal(repetida.estado, 422);
  assert.equal(repetida.cuerpo.problemas[0].codigo, 'cedula_repetida');

  baseFalsa(base());
  const vencido = await registrar(ENV, { ...FORMULARIO, fecha_vencimiento_praind: '2020-01-01' });
  assert.equal(vencido.cuerpo.problemas[0].codigo, 'praind_vencido');
});

test('registrar: si otro la registra a la vez, el índice único lo frena y se avisa igual', async () => {
  baseFalsa((texto) =>
    texto.startsWith('BEGIN')
      ? [
          {
            status: 'ERR',
            result: "Database index `contratista_cedula_unica` already contains '111111111'",
          },
        ]
      : base()(texto),
  );
  const { estado, cuerpo } = await registrar(ENV, FORMULARIO);
  assert.equal(estado, 422);
  assert.equal(cuerpo.problemas[0].codigo, 'cedula_repetida');
});

test('editar: sólo se guarda lo que cambió y la auditoría lo dice', async () => {
  const llamadas = baseFalsa(base());
  const { estado, cuerpo } = await editar(ENV, ID, { ...FORMULARIO, tiene_acceso: false });

  assert.equal(estado, 200);
  assert.equal(cuerpo.contratista.tiene_acceso, false);
  const [escritura] = escrituras(llamadas);
  assert.match(escritura.texto, /UPDATE type::record\('contratista'/);
  assert.equal(escritura.variables.accion, 'edicion');
  assert.deepEqual(
    escritura.variables.cambios.map((c) => c.campo),
    ['tiene_acceso'],
  );
});

test('editar sin cambiar nada no escribe', async () => {
  const llamadas = baseFalsa(base());
  const { estado } = await editar(ENV, ID, {
    ...FORMULARIO,
    cedula: '111111111',
    nombre: 'JOSE PEÑA',
  });
  assert.equal(estado, 200);
  assert.equal(escrituras(llamadas).length, 0);
});

test('editar: con el PRAIND vencido se le puede quitar el acceso, pero no cambiar la fecha a una vencida', async () => {
  const vencido = { ...JOSE, fecha_vencimiento_praind: '2020-01-01' };
  baseFalsa(base({ jose: [ok([vencido])] }));
  const quitar = await editar(ENV, ID, {
    ...FORMULARIO,
    fecha_vencimiento_praind: '2020-01-01',
    tiene_acceso: false,
  });
  assert.equal(quitar.estado, 200);

  const otraFecha = await editar(ENV, ID, {
    ...FORMULARIO,
    fecha_vencimiento_praind: '2021-01-01',
  });
  assert.equal(otraFecha.estado, 422);
  assert.equal(otraFecha.cuerpo.problemas[0].codigo, 'praind_vencido');
});

test('editar: no se cambia la cédula de quien está adentro', async () => {
  baseFalsa(base({ adentro: [{ id: 'presencia:⟨111111111⟩' }] }));
  const { estado, cuerpo } = await editar(ENV, ID, { ...FORMULARIO, cedula: '2-2222-2222' });
  assert.equal(estado, 422);
  assert.equal(cuerpo.problemas[0].codigo, 'cedula_no_editable_adentro');
});

test('editar: su propia cédula no cuenta como repetida; un ID raro o inexistente se rechaza', async () => {
  baseFalsa(base({ cedulaDe: [ok([ID])] }));
  assert.equal((await editar(ENV, ID, FORMULARIO)).estado, 200);

  assert.deepEqual(await editar(ENV, 'no-es-un-id', FORMULARIO), {
    estado: 400,
    cuerpo: { error: 'id_invalido' },
  });
  baseFalsa(base({ jose: [ok([])] }));
  assert.deepEqual(await editar(ENV, ID, FORMULARIO), {
    estado: 404,
    cuerpo: { error: 'no_encontrado' },
  });
});

test('listar: contratistas y empresas para la tabla y el selector', async () => {
  baseFalsa(() => [ok([JOSE]), ok([{ id: EMPRESA, nombre: 'ACME' }])]);
  const { estado, cuerpo } = await listar(ENV);
  assert.equal(estado, 200);
  assert.equal(cuerpo.contratistas.length, 1);
  assert.deepEqual(cuerpo.empresas, [{ id: EMPRESA, nombre: 'ACME' }]);
});

test('empresa: el núcleo la pone en mayúscula y no deja repetirla', async () => {
  const llamadas = baseFalsa(() => [ok([]), ok([])]);
  const nueva = await crearEmpresa(ENV, { nombre: '  acme   norte ' });
  assert.equal(nueva.estado, 201);
  assert.equal(nueva.cuerpo.empresa.nombre, 'ACME NORTE');
  const [escritura] = escrituras(llamadas);
  assert.equal(escritura.variables.nombre_busqueda, 'ACME NORTE');
  assert.equal(escritura.variables.entidad, 'empresa');

  baseFalsa((texto) => (texto.includes('FROM empresa WHERE nombre') ? [ok([EMPRESA])] : [ok([])]));
  const repetida = await crearEmpresa(ENV, { nombre: 'acme norte' });
  assert.equal(repetida.estado, 422);
  assert.equal(repetida.cuerpo.problemas[0].codigo, 'empresa_nombre_repetido');

  const vacia = await crearEmpresa(ENV, { nombre: '  ' });
  assert.equal(vacia.cuerpo.problemas[0].mensaje, 'El nombre de la empresa es obligatorio');
});
