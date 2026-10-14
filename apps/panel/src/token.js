// El acceso de un equipo a la nube. El equipo prueba quién es firmando un desafío con la
// clave privada que generó al vincularse (la pública quedó guardada en `dispositivo`); si
// sigue vinculado y no lo han retirado, recibe un token de la base. La clave de la base
// nunca sale del Worker. Estas funciones no tocan la red: se prueban solas (`token.test.mjs`).

const codificador = new TextEncoder();
/** Cuánto vale un desafío, en milisegundos. */
export const VIGENCIA_DESAFIO_MS = 120_000;

const base64url = (bytes) =>
  btoa(String.fromCharCode(...new Uint8Array(bytes)))
    .replaceAll('+', '-')
    .replaceAll('/', '_')
    .replaceAll('=', '');

const bytesDe = (texto) => {
  const relleno = texto.replaceAll('-', '+').replaceAll('_', '/');
  return Uint8Array.from(atob(relleno), (c) => c.charCodeAt(0));
};

/** HMAC del desafío con una clave derivada de un secreto del Worker (sin secretos nuevos). */
async function sello(secreto, mensaje) {
  const base = await crypto.subtle.importKey(
    'raw',
    codificador.encode(secreto),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  const derivada = await crypto.subtle.sign('HMAC', base, codificador.encode('limen-desafio'));
  const clave = await crypto.subtle.importKey(
    'raw',
    derivada,
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  return base64url(await crypto.subtle.sign('HMAC', clave, codificador.encode(mensaje)));
}

/** Un desafío para un equipo: no se guarda nada, lleva su propio sello y su vencimiento. */
export async function crearDesafio(secreto, equipo, ahora = Date.now()) {
  const azar = base64url(crypto.getRandomValues(new Uint8Array(16)));
  const cuerpo = `${equipo}.${ahora + VIGENCIA_DESAFIO_MS}.${azar}`;
  return `${cuerpo}.${await sello(secreto, cuerpo)}`;
}

/** ¿Es un desafío nuestro, de este equipo y todavía vigente? */
export async function desafioValido(secreto, desafio, equipo, ahora = Date.now()) {
  const partes = String(desafio ?? '').split('.');
  if (partes.length !== 4) return false;
  const [quien, vence, azar, firmado] = partes;
  if (quien !== equipo || !(Number(vence) > ahora)) return false;
  const esperado = await sello(secreto, `${quien}.${vence}.${azar}`);
  return esperado === firmado;
}

/** ¿Firmó este desafío el dueño de esa clave pública? (ECDSA P-256, SHA-256, r y s juntos). */
export async function firmaValida(jwk, desafio, firma) {
  try {
    const clave = await crypto.subtle.importKey(
      'jwk',
      jwk,
      { name: 'ECDSA', namedCurve: 'P-256' },
      false,
      ['verify'],
    );
    return await crypto.subtle.verify(
      { name: 'ECDSA', hash: 'SHA-256' },
      clave,
      bytesDe(String(firma)),
      codificador.encode(String(desafio)),
    );
  } catch {
    return false;
  }
}

/** Cuánto vale el token de un equipo, en segundos: pasado ese tiempo hay que pedir otro. */
export const VIGENCIA_TOKEN_S = 3600;

/**
 * El token con que un equipo entra a la base: un JWT firmado con HMAC-SHA512 que lleva su unidad
 * (la base sólo le deja ver y escribir lo de esa unidad) y su ID. Como dura una hora, un equipo
 * retirado pierde el acceso a más tardar en ese tiempo, al no poder renovarlo.
 */
export async function firmarJwt(secreto, { unidad, equipo }, ahora = Date.now()) {
  const iat = Math.floor(ahora / 1000);
  const cabecera = base64url(codificador.encode(JSON.stringify({ alg: 'HS512', typ: 'JWT' })));
  const cuerpo = base64url(
    codificador.encode(
      JSON.stringify({
        ns: 'limen',
        db: 'principal',
        ac: 'equipo',
        unidad,
        equipo,
        iat,
        exp: iat + VIGENCIA_TOKEN_S,
      }),
    ),
  );
  const clave = await crypto.subtle.importKey(
    'raw',
    codificador.encode(secreto),
    { name: 'HMAC', hash: 'SHA-512' },
    false,
    ['sign'],
  );
  const firma = await crypto.subtle.sign(
    'HMAC',
    clave,
    codificador.encode(`${cabecera}.${cuerpo}`),
  );
  return `${cabecera}.${cuerpo}.${base64url(firma)}`;
}

/**
 * ¿Es este token uno que firmamos nosotros y sigue vigente? Devuelve lo que lleva (`unidad`,
 * `equipo`) o `null`. Es el camino inverso de `firmarJwt`: así un equipo vinculado prueba quién
 * es ante el Worker sin repetir la firma del desafío en cada pregunta.
 */
export async function verificarJwt(secreto, token, ahora = Date.now()) {
  const partes = String(token ?? '').split('.');
  if (partes.length !== 3) return null;
  try {
    const clave = await crypto.subtle.importKey(
      'raw',
      codificador.encode(secreto),
      { name: 'HMAC', hash: 'SHA-512' },
      false,
      ['verify'],
    );
    const firmado = await crypto.subtle.verify(
      'HMAC',
      clave,
      bytesDe(partes[2]),
      codificador.encode(`${partes[0]}.${partes[1]}`),
    );
    if (!firmado) return null;
    const cuerpo = JSON.parse(new TextDecoder().decode(bytesDe(partes[1])));
    if (!(cuerpo.exp > Math.floor(ahora / 1000)) || cuerpo.ac !== 'equipo') return null;
    return { unidad: cuerpo.unidad, equipo: cuerpo.equipo };
  } catch {
    return null;
  }
}
