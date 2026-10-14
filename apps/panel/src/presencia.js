// Regla A9: antes de registrar una entrada, el equipo pregunta si esa persona ya está adentro en
// otra unidad. Un equipo sólo ve lo de su unidad en la base, así que la pregunta pasa por el
// Worker, que sí puede mirar todas. Sólo responde dónde está y desde cuándo, nada más de la
// persona. Quién decide qué hacer con la respuesta es el dominio (`presencia.rs`).

import { consultar, resultado, LECTURA } from './nube.js';
import { verificarJwt } from './token.js';

// Una cédula, o `KOF-` y el código de empleado: la misma clave que usa la tabla `presencia`.
const CLAVE = /^[A-Za-z0-9-]{1,40}$/;

const rechazo = (estado, error) => ({ estado, cuerpo: { error } });

/** `GET /api/presencia/:clave` con el token del equipo como Bearer. */
export async function presencia(env, cabecera, clave) {
  const equipo = await verificarJwt(
    env.TOKEN_SECRETO,
    String(cabecera ?? '').replace(/^Bearer /, ''),
  );
  if (!equipo) return rechazo(401, 'dispositivo_invalido');
  if (!CLAVE.test(String(clave ?? ''))) return rechazo(400, 'datos_invalidos');

  const [leido] = await consultar(
    env,
    LECTURA,
    `SELECT unidad, via, desde FROM ONLY type::record('presencia', <string>$clave);`,
    { clave },
  );
  const adentro = resultado(leido, null) ?? null;
  return { estado: 200, cuerpo: { adentro } };
}
