// Usuarios del panel: el Worker sólo orquesta; lo que se decide lo decide el núcleo (el
// WebAssembly de verdad). La base se simula con un `fetch` falso que anota lo que se le pide.
// Se corre con: node --import ./apps/panel/test/wasm.mjs --test apps/panel/test/usuarios.test.mjs
import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';
import {
  ADMINISTRADOR_DEL_PANEL,
  crear,
  editar,
  listar,
  restablecerClave,
} from '../src/usuarios.js';

const ID = '0199c7a2-0000-7000-8000-000000000001';
const OTRO = '0199c7a2-0000-7000-8000-000000000002';
const ENV = {
  SURREAL_URL: 'https://nube.prueba',
  SURREAL_USUARIO: 'lector',
  SURREAL_CLAVE: 'x',
  SURREAL_ADMIN_USUARIO: 'admin',
  SURREAL_ADMIN_CLAVE: 'y',
};
const ANA = {
  id: ID,
  cedula: '111111111',
  nombre: 'ANA MORA',
  activo: true,
  debe_cambiar_clave: false,
};

const original = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = original;
});

const ok = (result) => ({ status: 'OK', result });

/**
 * Una base de mentira. `responder(texto, variables)` devuelve el resultado de cada sentencia;
 * `llamadas` guarda lo que se le pidió, para comprobar qué se escribió.
 */
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

/** Responde como una base donde `ana` existe y la cédula ya está en manos de `dueno`. */
const conAna =
  (dueno = []) =>
  (texto) => {
    if (texto.includes('FROM ONLY')) return [ok(ANA)];
    if (texto.includes('WHERE cedula')) return [ok(dueno)];
    return [ok([]), ok([])];
  };

test('crear: el núcleo normaliza la cédula y el nombre y la clave temporal se muestra una vez', async () => {
  const llamadas = baseFalsa((texto) =>
    texto.includes('WHERE cedula') ? [ok([])] : [ok([]), ok([])],
  );
  const { estado, cuerpo } = await crear(ENV, { cedula: '1-1111-1111', nombre: '  ana   mora ' });

  assert.equal(estado, 201);
  assert.equal(cuerpo.usuario.cedula, '111111111');
  assert.equal(cuerpo.usuario.nombre, 'ANA MORA');
  assert.match(cuerpo.clave_temporal, /^[2-9A-HJ-NP-Z]{10}$/, 'diez símbolos sin 0, O, 1 ni I');

  const [escritura] = escrituras(llamadas);
  assert.match(escritura.texto, /crypto::argon2::generate\(\$clave\)/, 'se guarda cifrada');
  assert.equal(escritura.variables.clave, cuerpo.clave_temporal);
  assert.equal(escritura.variables.accion, 'alta');
  assert.equal(escritura.variables.operador, ADMINISTRADOR_DEL_PANEL);
  assert.deepEqual(
    escritura.variables.cambios.map((c) => c.campo),
    ['cedula', 'nombre', 'activo', 'debe_cambiar_clave'],
  );
  assert.ok(
    !JSON.stringify(escritura.variables.cambios).includes(cuerpo.clave_temporal),
    'la clave no va a la auditoría',
  );
});

test('crear: con la cédula repetida el núcleo la rechaza y no se escribe nada', async () => {
  const llamadas = baseFalsa((texto) => (texto.includes('WHERE cedula') ? [ok([OTRO])] : [ok([])]));
  const { estado, cuerpo } = await crear(ENV, { cedula: '1-1111-1111', nombre: 'ana' });
  assert.equal(estado, 409);
  assert.equal(cuerpo.error, 'usuario_cedula_repetida');
  assert.equal(escrituras(llamadas).length, 0);
});

test('crear: lo que el núcleo no acepta ni llega a la nube', async () => {
  const llamadas = baseFalsa(() => assert.fail('no debía tocar la nube'));
  for (const [datos, codigo] of [
    [{ cedula: '', nombre: 'ana' }, 'cedula_vacia'],
    [{ cedula: 'AB-12#', nombre: 'ana' }, 'cedula_invalida'],
    [{ cedula: '1-1111-1111', nombre: ' ' }, 'nombre_vacio'],
    [{ cedula: '1-1111-1111', nombre: 'ana 2' }, 'nombre_invalido'],
    [undefined, 'cedula_vacia'],
  ]) {
    const { estado, cuerpo } = await crear(ENV, datos);
    assert.equal(estado, 400);
    assert.equal(cuerpo.error, codigo);
  }
  assert.equal(llamadas.length, 0);
});

test('crear: si dos altas chocan en el índice único, también es cédula repetida', async () => {
  baseFalsa((texto) =>
    texto.includes('WHERE cedula')
      ? [ok([])]
      : [
          {
            status: 'ERR',
            result: "Database index `usuario_cedula_unica` already contains '111111111'",
          },
        ],
  );
  const { estado, cuerpo } = await crear(ENV, { cedula: '111111111', nombre: 'ana' });
  assert.equal(estado, 409);
  assert.equal(cuerpo.error, 'usuario_cedula_repetida');
});

test('editar: cambia la cédula y el nombre, los normaliza el núcleo y queda en la auditoría', async () => {
  const llamadas = baseFalsa(conAna());
  const { estado, cuerpo } = await editar(ENV, ID, {
    cedula: '2-2222-2222',
    nombre: 'ana mora vega',
    activo: true,
  });
  assert.equal(estado, 200);
  assert.equal(cuerpo.usuario.cedula, '222222222');
  assert.equal(cuerpo.usuario.nombre, 'ANA MORA VEGA');

  const [escritura] = escrituras(llamadas);
  assert.equal(escritura.variables.accion, 'edicion');
  assert.deepEqual(
    escritura.variables.cambios.map((c) => [c.campo, c.antes, c.despues]),
    [
      ['cedula', '111111111', '222222222'],
      ['nombre', 'ANA MORA', 'ANA MORA VEGA'],
    ],
  );
});

test('editar: dar de baja y reactivar es cambiar «activo»; no se borra a nadie', async () => {
  const llamadas = baseFalsa(conAna());
  const { estado, cuerpo } = await editar(ENV, ID, {
    cedula: '111111111',
    nombre: 'ana mora',
    activo: false,
  });
  assert.equal(estado, 200);
  assert.equal(cuerpo.usuario.activo, false);
  const [escritura] = escrituras(llamadas);
  assert.deepEqual(
    escritura.variables.cambios.map((c) => c.campo),
    ['activo'],
  );
  assert.ok(!escritura.texto.includes('DELETE'));
});

test('editar: la cédula de otro usuario se rechaza; la propia no cuenta como repetida', async () => {
  baseFalsa(conAna([OTRO]));
  const choque = await editar(ENV, ID, { cedula: '2-2222-2222', nombre: 'ana mora', activo: true });
  assert.equal(choque.estado, 409);
  assert.equal(choque.cuerpo.error, 'usuario_cedula_repetida');

  baseFalsa(conAna([ID]));
  const propia = await editar(ENV, ID, { cedula: '1-1111-1111', nombre: 'ana mora', activo: true });
  assert.equal(propia.estado, 200);
});

test('editar: sin cambios no se escribe ni se audita nada', async () => {
  const llamadas = baseFalsa(conAna([ID]));
  const { estado } = await editar(ENV, ID, {
    cedula: '111111111',
    nombre: 'Ana Mora',
    activo: true,
  });
  assert.equal(estado, 200);
  assert.equal(escrituras(llamadas).length, 0);
});

test('editar: un ID mal hecho, un usuario que no existe o un «activo» que no es sí/no', async () => {
  baseFalsa(() => assert.fail('no debía tocar la nube'));
  assert.equal(
    (await editar(ENV, 'no-es-un-id', { cedula: '1', nombre: 'a', activo: true })).cuerpo.error,
    'id_invalido',
  );
  assert.equal(
    (await editar(ENV, ID, { cedula: '1', nombre: 'a', activo: 'si' })).cuerpo.error,
    'datos_invalidos',
  );

  baseFalsa(() => [ok(null)]);
  const ausente = await editar(ENV, ID, { cedula: '111111111', nombre: 'ana', activo: true });
  assert.equal(ausente.estado, 404);
  assert.equal(ausente.cuerpo.error, 'no_encontrado');
});

test('restablecer la clave: otra clave temporal, cifrada, y la auditoría sólo dice que cambió', async () => {
  const llamadas = baseFalsa(conAna());
  const { estado, cuerpo } = await restablecerClave(ENV, ID);
  assert.equal(estado, 200);
  assert.match(cuerpo.clave_temporal, /^[2-9A-HJ-NP-Z]{10}$/);

  const [escritura] = escrituras(llamadas);
  assert.match(escritura.texto, /debe_cambiar_clave = true/);
  assert.match(escritura.texto, /crypto::argon2::generate\(\$clave\)/);
  assert.equal(escritura.variables.cambios[0].campo, 'clave');
  assert.equal(escritura.variables.cambios[0].despues, 'cambiada');
  assert.ok(!JSON.stringify(escritura.variables.cambios).includes(cuerpo.clave_temporal));
});

test('listar: trae los usuarios sin su clave', async () => {
  const llamadas = baseFalsa(() => [ok([ANA])]);
  const { estado, cuerpo } = await listar(ENV);
  assert.equal(estado, 200);
  assert.deepEqual(cuerpo.usuarios, [ANA]);
  assert.ok(
    !/\bclave\b\s*[,F]/.test(llamadas[0].texto.replace('debe_cambiar_clave', '')),
    'no pide la clave',
  );
});
