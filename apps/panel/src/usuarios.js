// Usuarios (bloque L): crear, editar (cédula, nombre y activo; activar o desactivar, nunca
// borrar) y restablecer la clave. Los usuarios son globales y los administra la nube; el equipo sólo los
// baja. Aquí sólo se orquesta: leer, guardar y anotar la auditoría. **Todo lo que se decide lo
// decide el núcleo** (`dominio.js`): qué cédula y qué nombre valen y cómo quedan normalizados,
// que la cédula sea única, qué cambió y cómo es la clave temporal.

import {
  clave_temporal,
  usuario_para_alta,
  usuario_para_edicion,
  usuario_para_restablecer,
} from './dominio.js';
import { consultar, resultado, LECTURA, ESCRITURA } from './nube.js';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Quién hizo el cambio en la auditoría. Mientras el panel entre con una clave compartida no
 * hay un usuario con nombre: se anota el identificador nulo. Con Cloudflare Access será
 * quien entró.
 */
export const ADMINISTRADOR_DEL_PANEL = '00000000-0000-0000-0000-000000000000';

const ESTADO_DE_LA_REGLA = {
  usuario_cedula_repetida: 409,
  no_encontrado: 404,
};

/** Un error con el código de la regla que lo impide (`cedula_invalida`, `nombre_vacio`…). */
class ErrorDeRegla extends Error {
  constructor(codigo) {
    super(codigo);
    this.codigo = codigo;
  }
}

/**
 * Corre una operación y traduce lo que no vale a la respuesta HTTP. El núcleo avisa con el
 * código de la regla como texto; cualquier otra cosa (la nube, un fallo nuestro) sigue de
 * largo y la atiende el manejador general del Worker.
 */
async function aplicar(operacion) {
  try {
    return await operacion();
  } catch (error) {
    const codigo =
      error instanceof ErrorDeRegla ? error.codigo : typeof error === 'string' ? error : null;
    if (codigo === null) throw error;
    return { estado: ESTADO_DE_LA_REGLA[codigo] ?? 400, cuerpo: { error: codigo } };
  }
}

const texto = (valor) => (typeof valor === 'string' ? valor : '');

function exigirId(id) {
  if (!UUID.test(String(id ?? ''))) throw new ErrorDeRegla('id_invalido');
}

/** Corre varias sentencias juntas: si alguna falla, ninguna queda. */
async function transaccion(env, sentencias, variables) {
  const resultados = await consultar(env, ESCRITURA, `BEGIN; ${sentencias} COMMIT;`, variables);
  for (const sentencia of resultados) {
    if (sentencia.status !== 'OK') {
      // El índice único de la cédula es la última defensa ante dos altas a la vez.
      if (String(sentencia.result).includes('usuario_cedula_unica')) {
        throw new ErrorDeRegla('usuario_cedula_repetida');
      }
      throw new Error('consulta');
    }
  }
}

const CAMPOS = 'record::id(id) AS id, cedula, nombre, activo, debe_cambiar_clave';

// El identificador de un usuario es un UUID: así lo guarda y lo lee el escritorio (`uuid_de`).
// Se busca comparando su texto, que sirve igual si alguna vez quedó guardado de otra forma.
const POR_ID = '<string>record::id(id) = <string>$id';

async function leer(env, id) {
  exigirId(id);
  const [s] = await consultar(
    env,
    LECTURA,
    `SELECT ${CAMPOS} FROM usuario WHERE ${POR_ID} LIMIT 1;`,
    { id },
  );
  const usuario = resultado(s)[0];
  if (!usuario) throw new ErrorDeRegla('no_encontrado');
  return usuario;
}

/** El ID del usuario que ya tiene esa cédula (ya normalizada), o `null`. */
async function quienTieneLaCedula(env, cedula) {
  const [s] = await consultar(
    env,
    LECTURA,
    'SELECT VALUE record::id(id) FROM usuario WHERE cedula = $cedula LIMIT 1;',
    { cedula },
  );
  return resultado(s)[0] ?? null;
}

const AUDITORIA = `CREATE auditoria SET entidad = 'usuario', registro = <string>$id, accion = $accion,
  cambios = $cambios, operador = <uuid>$operador, en = time::now();`;

const hashDeLaClave = 'crypto::argon2::generate($clave)';

export async function listar(env) {
  const [s] = await consultar(env, LECTURA, `SELECT ${CAMPOS} FROM usuario ORDER BY nombre;`);
  return { estado: 200, cuerpo: { usuarios: resultado(s) } };
}

/** Un usuario nuevo con una clave temporal, que se muestra una sola vez. */
export function crear(env, cuerpo) {
  return aplicar(async () => {
    const cedula = texto(cuerpo?.cedula);
    const nombre = texto(cuerpo?.nombre);
    // Primero el núcleo normaliza (la cédula única se busca con la cédula ya normalizada);
    // si resulta que ya existe, se le pregunta de nuevo, ahora con el hecho, y él decide.
    const previo = JSON.parse(usuario_para_alta(cedula, nombre, false));
    const repetida = (await quienTieneLaCedula(env, previo.cedula)) !== null;
    const alta = repetida ? JSON.parse(usuario_para_alta(cedula, nombre, true)) : previo;

    const id = crypto.randomUUID();
    const clave = clave_temporal(crypto.getRandomValues(new Uint8Array(10)), alta.cedula);
    await transaccion(
      env,
      `CREATE type::record('usuario', <uuid>$id) SET cedula = $cedula, nombre = $nombre,
         activo = true, clave = ${hashDeLaClave}, debe_cambiar_clave = true;
       ${AUDITORIA}`,
      {
        id,
        cedula: alta.cedula,
        nombre: alta.nombre,
        clave,
        accion: 'alta',
        cambios: alta.cambios,
        operador: ADMINISTRADOR_DEL_PANEL,
      },
    );
    return {
      estado: 201,
      cuerpo: {
        usuario: {
          id,
          cedula: alta.cedula,
          nombre: alta.nombre,
          activo: true,
          debe_cambiar_clave: true,
        },
        clave_temporal: clave,
      },
    };
  });
}

/** Cambia la cédula, el nombre y si está activo (activar o desactivar no borra a nadie). */
export function editar(env, id, cuerpo) {
  return aplicar(async () => {
    if (typeof cuerpo?.activo !== 'boolean') throw new ErrorDeRegla('datos_invalidos');
    const actual = await leer(env, id);
    const cedula = texto(cuerpo.cedula);
    const nombre = texto(cuerpo.nombre);
    const pedir = (enUso) =>
      JSON.parse(
        usuario_para_edicion(
          id,
          actual.cedula,
          actual.nombre,
          actual.activo,
          cedula,
          nombre,
          cuerpo.activo,
          enUso,
        ),
      );
    // Igual que al crear: se normaliza primero y después se pregunta con el hecho.
    const previo = pedir(false);
    const dueno = await quienTieneLaCedula(env, previo.cedula);
    const edicion = dueno !== null && dueno !== id ? pedir(true) : previo;

    if (edicion.cambios.length > 0) {
      await transaccion(
        env,
        `UPDATE usuario SET cedula = $cedula, nombre = $nombre, activo = $activo WHERE ${POR_ID};
         ${AUDITORIA}`,
        {
          id,
          cedula: edicion.cedula,
          nombre: edicion.nombre,
          activo: edicion.activo,
          accion: 'edicion',
          cambios: edicion.cambios,
          operador: ADMINISTRADOR_DEL_PANEL,
        },
      );
    }
    return {
      estado: 200,
      cuerpo: {
        usuario: {
          id,
          cedula: edicion.cedula,
          nombre: edicion.nombre,
          activo: edicion.activo,
          debe_cambiar_clave: actual.debe_cambiar_clave,
        },
      },
    };
  });
}

/** Le pone otra clave temporal (olvidó la suya): la persona la cambia al entrar. */
export function restablecerClave(env, id) {
  return aplicar(async () => {
    const actual = await leer(env, id);
    const { cambios } = JSON.parse(
      usuario_para_restablecer(
        id,
        actual.cedula,
        actual.nombre,
        actual.activo,
        actual.debe_cambiar_clave,
      ),
    );
    const clave = clave_temporal(crypto.getRandomValues(new Uint8Array(10)), actual.cedula);
    await transaccion(
      env,
      `UPDATE usuario SET clave = ${hashDeLaClave}, debe_cambiar_clave = true WHERE ${POR_ID};
       ${AUDITORIA}`,
      { id, clave, accion: 'edicion', cambios, operador: ADMINISTRADOR_DEL_PANEL },
    );
    return { estado: 200, cuerpo: { clave_temporal: clave } };
  });
}
