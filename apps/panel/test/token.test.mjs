// Pruebas de las partes del acceso que no tocan la red: `node --test apps/panel/test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { VIGENCIA_DESAFIO_MS, crearDesafio, desafioValido, firmaValida } from '../src/token.js';

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
