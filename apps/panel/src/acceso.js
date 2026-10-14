// El acceso de un equipo a la nube (`POST /api/token`), en dos pasos:
//   1. el equipo manda su ID y recibe un desafío;
//   2. lo devuelve firmado con su clave privada; si la firma es suya y el dominio dice que
//      puede conectarse (vinculado y no retirado, regla M2), recibe un token de la base.
// Lo que recibe el equipo es un token firmado con su unidad (`TOKEN_SECRETO`, que sólo conocen el
// Worker y la base): la base lo acepta y sólo le deja ver y escribir lo de esa unidad (`DEFINE ACCESS
// equipo`, docs/nube.md sección 13). Ninguna clave de la base sale del Worker. Aquí sólo se orquesta: las reglas son del dominio.

import { motivo_de_rechazo_del_equipo } from './dominio/limen_dominio_wasm.js';
import { consultar, resultado, LECTURA } from './nube.js';
import { crearDesafio, desafioValido, firmaValida, firmarJwt } from './token.js';
import { registrarEvento } from './vinculacion.js';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

const rechazo = (estado, error) => ({ estado, cuerpo: { error } });
// Todo lo que no sea un equipo vigente con su firma recibe la misma respuesta: no se dice
// cuál de las cosas falló.
const invalido = () => rechazo(401, 'dispositivo_invalido');

export async function acceso(env, { equipo, desafio, firma } = {}) {
  if (!UUID.test(String(equipo ?? ''))) return rechazo(400, 'datos_invalidos');
  const secreto = env.SURREAL_ADMIN_CLAVE;

  if (!desafio) {
    return { estado: 200, cuerpo: { desafio: await crearDesafio(secreto, equipo) } };
  }
  if (!(await desafioValido(secreto, desafio, equipo))) return invalido();

  const [leido] = await consultar(
    env,
    LECTURA,
    `SELECT clave_publica, unidad, vinculado, retirado FROM ONLY type::record('dispositivo', <string>$id);`,
    { id: equipo },
  );
  const dispositivo = resultado(leido, null);
  const motivo = motivo_de_rechazo_del_equipo(
    Boolean(dispositivo?.vinculado),
    Boolean(dispositivo?.retirado),
  );
  if (!dispositivo || motivo) {
    await registrarEvento(env, motivo || 'dispositivo_inexistente', `equipo ${equipo}`);
    return invalido();
  }
  if (!(await firmaValida(dispositivo.clave_publica, desafio, firma))) {
    await registrarEvento(env, 'firma_invalida', `equipo ${equipo}`);
    return invalido();
  }

  return {
    estado: 200,
    cuerpo: {
      token: await firmarJwt(env.TOKEN_SECRETO, {
        unidad: dispositivo.unidad,
        equipo,
      }),
      url: env.SURREAL_URL.replace(/^http/, 'ws'),
    },
  };
}
