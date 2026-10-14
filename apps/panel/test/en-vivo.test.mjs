// El token del tiempo real: de sólo lectura, de una hora y firmado con un secreto aparte del de los
// equipos. Se corre con: npm test (en apps/panel).
import assert from 'node:assert/strict';
import { createHmac } from 'node:crypto';
import { test } from 'node:test';
import { enVivo } from '../src/en_vivo.js';
import { VIGENCIA_TOKEN_S } from '../src/token.js';

const ENV = {
  SURREAL_URL: 'https://limen.aws-use1.surreal.cloud',
  PANEL_VIVO_SECRETO: 'secreto-del-panel',
};
const AHORA = Date.UTC(2026, 9, 14, 17, 0, 0);

const partes = (token) => token.split('.');
const datosDe = (token) => JSON.parse(Buffer.from(partes(token)[1], 'base64url').toString());

test('entrega un token del panel de una hora, firmado con el secreto del panel', async () => {
  const { estado, cuerpo } = await enVivo(ENV, AHORA);
  assert.equal(estado, 200);

  const [cabecera, datos, firma] = partes(cuerpo.token);
  const esperada = createHmac('sha512', ENV.PANEL_VIVO_SECRETO)
    .update(`${cabecera}.${datos}`)
    .digest('base64url');
  assert.equal(firma, esperada, 'la firma es HMAC-SHA512 con el secreto del panel');
  assert.equal(JSON.parse(Buffer.from(cabecera, 'base64url').toString()).alg, 'HS512');

  const claims = datosDe(cuerpo.token);
  assert.equal(claims.ac, 'panel', 'entra por el acceso del panel');
  assert.equal(claims.ns, 'limen');
  assert.equal(claims.db, 'principal');
  assert.equal(claims.exp - claims.iat, VIGENCIA_TOKEN_S, 'vale una hora');
  assert.equal(claims.unidad, undefined, 'no lleva la unidad de nadie');
  assert.equal(claims.equipo, undefined);
});

test('dice a dónde conectarse (WebSocket seguro) y cuándo vence', async () => {
  const { cuerpo } = await enVivo(ENV, AHORA);
  assert.equal(cuerpo.url, 'wss://limen.aws-use1.surreal.cloud');
  assert.equal(cuerpo.vence, AHORA + VIGENCIA_TOKEN_S * 1000);
});

test('no vale con el secreto de otro: los de los equipos no sirven para el panel', async () => {
  const { cuerpo } = await enVivo(ENV, AHORA);
  const [cabecera, datos, firma] = partes(cuerpo.token);
  const ajena = createHmac('sha512', 'secreto-de-los-equipos')
    .update(`${cabecera}.${datos}`)
    .digest('base64url');
  assert.notEqual(firma, ajena);
});

test('sin el secreto configurado, avisa que no hay tiempo real en vez de fallar', async () => {
  const { estado, cuerpo } = await enVivo({ SURREAL_URL: ENV.SURREAL_URL }, AHORA);
  assert.equal(estado, 503);
  assert.equal(cuerpo.error, 'en_vivo_sin_configurar');
  assert.equal(cuerpo.token, undefined);
});
