// Pruebas de las partes del acceso que no tocan la red: `node --test apps/panel/test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  VIGENCIA_DESAFIO_MS,
  VIGENCIA_TOKEN_S,
  crearDesafio,
  desafioValido,
  firmaValida,
  firmarJwt,
  verificarJwt,
} from '../src/token.js';

const EQUIPO = '9df9696d-a2e6-42fb-8359-3325fc844e66';
const SECRETO = 'secreto-de-prueba';
const base64url = (bytes) =>
  Buffer.from(bytes)
    .toString('base64')
    .replaceAll('+', '-')
    .replaceAll('/', '_')
    .replaceAll('=', '');

test('un desafío recién creado vale para su equipo', async () => {
  const desafio = await crearDesafio(SECRETO, EQUIPO);
  assert.equal(await desafioValido(SECRETO, desafio, EQUIPO), true);
});

test('no vale para otro equipo, con otro secreto ni alterado', async () => {
  const desafio = await crearDesafio(SECRETO, EQUIPO);
  assert.equal(await desafioValido(SECRETO, desafio, 'otro-equipo'), false);
  assert.equal(await desafioValido('otro-secreto', desafio, EQUIPO), false);
  assert.equal(await desafioValido(SECRETO, desafio.replace('.', 'x.'), EQUIPO), false);
  assert.equal(await desafioValido(SECRETO, undefined, EQUIPO), false);
});

test('vence a los dos minutos', async () => {
  const ahora = Date.now();
  const desafio = await crearDesafio(SECRETO, EQUIPO, ahora);
  assert.equal(
    await desafioValido(SECRETO, desafio, EQUIPO, ahora + VIGENCIA_DESAFIO_MS - 1),
    true,
  );
  assert.equal(await desafioValido(SECRETO, desafio, EQUIPO, ahora + VIGENCIA_DESAFIO_MS), false);
});

test('la firma del dueño de la clave vale y la de otro no', async () => {
  const par = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, [
    'sign',
    'verify',
  ]);
  const otro = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, [
    'sign',
    'verify',
  ]);
  const jwk = await crypto.subtle.exportKey('jwk', par.publicKey);
  const desafio = await crearDesafio(SECRETO, EQUIPO);
  const firmar = async (clave) =>
    base64url(
      await crypto.subtle.sign(
        { name: 'ECDSA', hash: 'SHA-256' },
        clave,
        new TextEncoder().encode(desafio),
      ),
    );

  assert.equal(await firmaValida(jwk, desafio, await firmar(par.privateKey)), true);
  assert.equal(await firmaValida(jwk, desafio, await firmar(otro.privateKey)), false);
  assert.equal(await firmaValida(jwk, desafio + 'x', await firmar(par.privateKey)), false);
  assert.equal(await firmaValida(jwk, desafio, 'no-es-una-firma'), false);
});

test('el token de un equipo es un JWT HS512 con su unidad, firmado y de una hora', async () => {
  const ahora = Date.UTC(2026, 9, 13, 12, 0, 0);
  const token = await firmarJwt(SECRETO, { unidad: 'BRISAS', equipo: EQUIPO }, ahora);

  const [cabecera, cuerpo, firma] = token.split('.');
  const leer = (parte) => JSON.parse(Buffer.from(parte, 'base64url').toString());
  assert.deepEqual(leer(cabecera), { alg: 'HS512', typ: 'JWT' });
  const datos = leer(cuerpo);
  assert.equal(datos.unidad, 'BRISAS');
  assert.equal(datos.equipo, EQUIPO);
  assert.equal(datos.ac, 'equipo');
  assert.equal(datos.exp - datos.iat, VIGENCIA_TOKEN_S);

  const clave = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(SECRETO),
    { name: 'HMAC', hash: 'SHA-512' },
    false,
    ['verify'],
  );
  const valida = (mensaje) =>
    crypto.subtle.verify(
      'HMAC',
      clave,
      Buffer.from(firma, 'base64url'),
      new TextEncoder().encode(mensaje),
    );
  assert.equal(await valida(`${cabecera}.${cuerpo}`), true, 'la firma es del Worker');
  assert.equal(await valida(`${cabecera}.${cuerpo}x`), false, 'y no vale con otro contenido');
});

test('el token que firmamos se verifica y devuelve su unidad y su equipo', async () => {
  const token = await firmarJwt(SECRETO, { unidad: 'BRISAS', equipo: EQUIPO });
  assert.deepEqual(await verificarJwt(SECRETO, token), { unidad: 'BRISAS', equipo: EQUIPO });
});

test('un token ajeno, alterado, vencido o ilegible no se verifica', async () => {
  const ahora = Date.now();
  const token = await firmarJwt(SECRETO, { unidad: 'BRISAS', equipo: EQUIPO }, ahora);
  assert.equal(await verificarJwt('otro-secreto', token, ahora), null);
  const [cabecera, cuerpo, firma] = token.split('.');
  const otro = base64url(
    JSON.stringify({ ...JSON.parse(Buffer.from(cuerpo, 'base64url')), unidad: 'CARTAGO' }),
  );
  assert.equal(await verificarJwt(SECRETO, `${cabecera}.${otro}.${firma}`, ahora), null);
  assert.equal(await verificarJwt(SECRETO, token, ahora + (VIGENCIA_TOKEN_S + 1) * 1000), null);
  assert.equal(await verificarJwt(SECRETO, 'basura', ahora), null);
  assert.equal(await verificarJwt(SECRETO, undefined, ahora), null);
});
