// Cliente mínimo de SurrealDB por HTTP (/signin y /rpc). Los datos van siempre como
// parámetros, nunca pegados al texto de la consulta.

export const NAMESPACE = 'limen';
export const BASE = 'principal';

// SurrealDB no acepta a un usuario de base de datos con Basic auth: se entra por
// /signin y se usa el token (dura 1 h). Se guarda por usuario mientras viva el Worker.
const sesiones = new Map();

async function iniciarSesion(env, usuario, clave) {
  const respuesta = await fetch(`${env.SURREAL_URL}/signin`, {
    method: 'POST',
    headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: JSON.stringify({ ns: NAMESPACE, db: BASE, user: usuario, pass: clave }),
  });
  const cuerpo = await respuesta.json();
  if (!cuerpo.token) throw new Error('signin');
  return cuerpo.token;
}

async function token(env, usuario, clave) {
  const guardada = sesiones.get(usuario);
  if (guardada && Date.now() < guardada.hasta) return guardada.token;
  const nuevo = await iniciarSesion(env, usuario, clave);
  sesiones.set(usuario, { token: nuevo, hasta: Date.now() + 50 * 60 * 1000 });
  return nuevo;
}

/** Un token recién emitido (no el guardado): el de un equipo que acaba de probar quién es. */
export function tokenNuevo(env, rol) {
  const [usuario, clave] = rol(env);
  return iniciarSesion(env, usuario, clave);
}

/** Quién consulta: el usuario de sólo lectura o el que puede escribir. */
export const LECTURA = (env) => [env.SURREAL_USUARIO, env.SURREAL_CLAVE];
export const ESCRITURA = (env) => [env.SURREAL_ADMIN_USUARIO, env.SURREAL_ADMIN_CLAVE];

/** Corre una consulta y devuelve el resultado de cada sentencia. */
export async function consultar(env, rol, texto, variables = {}) {
  const [usuario, clave] = rol(env);
  const respuesta = await fetch(`${env.SURREAL_URL}/rpc`, {
    method: 'POST',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
      'Surreal-NS': NAMESPACE,
      'Surreal-DB': BASE,
      Authorization: 'Bearer ' + (await token(env, usuario, clave)),
    },
    body: JSON.stringify({ id: 1, method: 'query', params: [texto, variables] }),
  });
  const cuerpo = await respuesta.json();
  if (cuerpo.error) throw new Error(cuerpo.error.message);
  return cuerpo.result;
}

/** El resultado de una sentencia, o error si falló (salvo que la tabla no exista aún). */
export function resultado(sentencia, siFalta = []) {
  if (sentencia?.status === 'ERR' && sentencia.result?.includes?.('does not exist')) return siFalta;
  if (sentencia?.status !== 'OK') throw new Error('consulta');
  return sentencia.result;
}
