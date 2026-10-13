// Registrar, listar, retirar y canjear. Aquí sólo se orquesta: guardar, firmar y
// responder. Las reglas del código (formato, vigencia, un solo uso) las decide el
// dominio de Rust, compilado a WebAssembly (`dominio/`, hecho por `npm run dominio`).

import {
  initSync,
  generar_codigo,
  leer_codigo,
  codigo_para_mostrar,
  motivo_de_rechazo,
  vigencia_minutos,
} from './dominio/limen_dominio_wasm.js';
import modulo from './dominio/limen_dominio_wasm_bg.wasm';
import { consultar, resultado, LECTURA, ESCRITURA } from './nube.js';

initSync({ module: modulo });

const TIPOS = ['pc', 'celular', 'visor'];

const hex = (bytes) => [...new Uint8Array(bytes)].map((b) => b.toString(16).padStart(2, '0')).join('');
const base64url = (bytes) =>
  btoa(String.fromCharCode(...new Uint8Array(bytes)))
    .replaceAll('+', '-')
    .replaceAll('/', '_')
    .replaceAll('=', '');

/** Lo que se guarda del código: su SHA-256, nunca el código. */
async function huellaDelCodigo(codigoCanonico) {
  return hex(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(codigoCanonico)));
}

/** Huella de la clave pública (RFC 7638): SHA-256 del JWK canónico. Valida la clave. */
async function huellaDeLaClave(jwk) {
  if (jwk?.kty !== 'EC' || jwk?.crv !== 'P-256') throw new Error('clave_invalida');
  await crypto.subtle.importKey('jwk', jwk, { name: 'ECDSA', namedCurve: 'P-256' }, false, ['verify']);
  const canonico = JSON.stringify({ crv: jwk.crv, kty: jwk.kty, x: jwk.x, y: jwk.y });
  return base64url(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(canonico)));
}

async function evento(env, tipo, detalle) {
  await consultar(env, ESCRITURA, 'CREATE evento_seguridad SET tipo = $tipo, detalle = $detalle, en = time::now();', {
    tipo,
    detalle,
  }).catch(() => {});
}

/** Registra un equipo y emite su código. */
export async function registrar(env, { nombre, unidad, tipo }) {
  if (!nombre?.trim() || !unidad?.trim() || !TIPOS.includes(tipo)) {
    return { estado: 400, cuerpo: { error: 'datos_invalidos' } };
  }
  const id = crypto.randomUUID();
  const codigo = generar_codigo(crypto.getRandomValues(new Uint8Array(10)));
  const huella = await huellaDelCodigo(codigo);
  const hechas = await consultar(
    env,
    ESCRITURA,
    `BEGIN;
     CREATE type::record('dispositivo', $id) SET nombre = $nombre, unidad = $unidad, tipo = $tipo, creado = time::now();
     CREATE type::record('codigo_vinculacion', $huella) SET dispositivo = $id, emitido = time::now(), usado = false, anulado = false;
     COMMIT;`,
    { id, nombre: nombre.trim(), unidad: unidad.trim().toUpperCase(), tipo, huella },
  );
  hechas.forEach((s) => resultado(s));
  return {
    estado: 201,
    cuerpo: { id, codigo: codigo_para_mostrar(codigo), vigencia_minutos: vigencia_minutos() },
  };
}

/** Los equipos con su estado de vinculación. */
export async function listar(env) {
  const [s] = await consultar(
    env,
    LECTURA,
    `SELECT record::id(id) AS id, nombre, unidad, tipo, creado, vinculado, retirado, huella FROM dispositivo ORDER BY creado DESC;`,
  );
  return resultado(s).map((d) => ({
    ...d,
    huella: undefined,
    estado: d.retirado ? 'retirado' : d.vinculado ? 'vinculado' : 'esperando_codigo',
  }));
}

/** Retira un equipo: pierde el acceso y su código pendiente se anula (regla M7). */
export async function retirar(env, id) {
  const hechas = await consultar(
    env,
    ESCRITURA,
    `BEGIN;
     UPDATE type::record('dispositivo', $id) SET retirado = time::now() WHERE retirado = NONE;
     UPDATE codigo_vinculacion SET anulado = true WHERE dispositivo = $id AND usado = false;
     COMMIT;`,
    { id },
  );
  hechas.forEach((s) => resultado(s));
  return { estado: 200, cuerpo: { ok: true } };
}

/** El equipo canjea su código y deja su clave pública. */
export async function canjear(env, { codigo, clave_publica: clave }) {
  let canonico;
  try {
    canonico = leer_codigo(String(codigo ?? ''));
  } catch (motivo) {
    await evento(env, String(motivo), 'formato del código');
    return { estado: 400, cuerpo: { error: String(motivo) } };
  }
  const huellaCodigo = await huellaDelCodigo(canonico);
  const [leido] = await consultar(
    env,
    LECTURA,
    `SELECT record::id(id) AS huella, dispositivo, emitido, usado, anulado FROM ONLY type::record('codigo_vinculacion', $huella);`,
    { huella: huellaCodigo },
  );
  const fila = resultado(leido, null);
  if (!fila) {
    await evento(env, 'codigo_inexistente', 'huella ' + huellaCodigo.slice(0, 8));
    return { estado: 404, cuerpo: { error: 'codigo_inexistente' } };
  }
  const motivo = motivo_de_rechazo(Date.parse(fila.emitido), fila.usado, fila.anulado, Date.now());
  if (motivo) {
    await evento(env, motivo, 'dispositivo ' + fila.dispositivo);
    return { estado: 400, cuerpo: { error: motivo } };
  }
  let huellaClave;
  try {
    huellaClave = await huellaDeLaClave(clave);
  } catch {
    return { estado: 400, cuerpo: { error: 'clave_invalida' } };
  }
  // Se marca como usado con una condición: si dos equipos canjean a la vez, gana uno.
  const hechas = await consultar(
    env,
    ESCRITURA,
    `LET $marcado = UPDATE type::record('codigo_vinculacion', $huellaCodigo) SET usado = true WHERE usado = false AND anulado = false;
     IF array::len($marcado) = 0 { THROW 'codigo_usado' };
     UPDATE type::record('dispositivo', $id) SET huella = $huellaClave, clave_publica = $clave, vinculado = time::now() WHERE retirado = NONE AND vinculado = NONE;`,
    { huellaCodigo, id: fila.dispositivo, huellaClave, clave },
  );
  if (hechas.some((s) => s.status === 'ERR')) {
    await evento(env, 'codigo_usado', 'dispositivo ' + fila.dispositivo);
    return { estado: 400, cuerpo: { error: 'codigo_usado' } };
  }
  return { estado: 200, cuerpo: { equipo: fila.dispositivo, huella: huellaClave } };
}
