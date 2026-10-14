// Lo común de las pantallas que escriben en la nube (usuarios, contratistas…): el error con
// el código de la regla, la respuesta HTTP que le toca y la transacción. Aquí no se decide
// ninguna regla de negocio: eso es del núcleo (`dominio.js`).

import { consultar, ESCRITURA } from './nube.js';

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
export class ErrorDeRegla extends Error {
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
export async function aplicar(operacion) {
  try {
    return await operacion();
  } catch (error) {
    const codigo =
      error instanceof ErrorDeRegla ? error.codigo : typeof error === 'string' ? error : null;
    if (codigo === null) throw error;
    return { estado: ESTADO_DE_LA_REGLA[codigo] ?? 400, cuerpo: { error: codigo } };
  }
}

export const texto = (valor) => (typeof valor === 'string' ? valor : '');

export function exigirId(id) {
  if (!UUID.test(String(id ?? ''))) throw new ErrorDeRegla('id_invalido');
}

/**
 * Corre varias sentencias juntas: si alguna falla, ninguna queda. `indices` dice, por el
 * nombre de cada índice único de la base, el código de la regla que se avisa si lo rechaza:
 * es la última defensa ante dos altas a la vez.
 */
export async function transaccion(env, sentencias, variables, indices = {}) {
  const resultados = await consultar(env, ESCRITURA, `BEGIN; ${sentencias} COMMIT;`, variables);
  // Al fallar una sentencia, la base marca las demás como «no ejecutadas»: la causa es la otra.
  const fallo = resultados.find(
    (s) => s.status !== 'OK' && !String(s.result).includes('failed transaction'),
  );
  if (fallo) {
    for (const [indice, codigo] of Object.entries(indices)) {
      if (String(fallo.result).includes(indice)) throw new ErrorDeRegla(codigo);
    }
    // El motivo queda en los registros del Worker; al navegador sólo llega el código.
    throw new Error(`consulta: ${String(fallo.result).slice(0, 300)}`);
  }
  if (resultados.some((s) => s.status !== 'OK')) throw new Error('consulta');
}
