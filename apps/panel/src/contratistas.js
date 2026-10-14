// Contratistas y sus empresas (catálogo global): listar, registrar y editar. Aquí sólo se
// orquesta: leer lo que la regla necesita saber (¿otro tiene la cédula?, ¿la empresa existe?,
// ¿está adentro?), guardar y anotar la auditoría. **Todo lo que se decide lo decide el
// núcleo** (`dominio.js`): qué cédula y nombre valen y cómo quedan, el PRAIND vigente, los
// mensajes de cada campo y qué cambió. Un contratista nunca se borra: se le quita el acceso.

import {
  contratista_cedula,
  contratista_evaluar,
  empresa_evaluar,
  empresa_nombre,
} from './dominio.js';
import { consultar, resultado, LECTURA } from './nube.js';
import {
  ADMINISTRADOR_DEL_PANEL,
  aplicar,
  ErrorDeRegla,
  exigirId,
  texto,
  transaccion,
} from './escritura.js';

const INDICES = {
  contratista_cedula_unica: 'contratista_cedula_unica',
  empresa_nombre_unico: 'empresa_nombre_unico',
};

// Los identificadores son UUID (así los guarda y lee el escritorio); se comparan por su texto.
const POR_ID = '<string>record::id(id) = <string>$id';

const CONTRATISTA =
  'record::id(id) AS id, cedula, nombre, <string>record::id(empresa) AS empresa_id, ' +
  'empresa.nombre AS empresa_nombre, tipo_ingreso, ' +
  "time::format(fecha_vencimiento_praind, '%Y-%m-%d') AS fecha_vencimiento_praind, tiene_acceso";

// La base lee como UUID todo texto que lo parezca (la empresa en el alta): se devuelve a texto,
// que es lo que la auditoría guarda.
const AUDITORIA = `CREATE auditoria SET entidad = $entidad, registro = <string>$id, accion = $accion,
  cambios = $cambios.map(|$c| { campo: $c.campo, antes: <string>$c.antes, despues: <string>$c.despues }),
  operador = <uuid>$operador, en = time::now();`;

const dueno = async (env, consulta, variables) => {
  const [s] = await consultar(env, LECTURA, consulta, variables);
  return resultado(s)[0] ?? null;
};

export async function listar(env) {
  const [c, e] = await consultar(
    env,
    LECTURA,
    `SELECT ${CONTRATISTA} FROM contratista ORDER BY nombre; ` +
      'SELECT record::id(id) AS id, nombre FROM empresa ORDER BY nombre;',
  );
  return { estado: 200, cuerpo: { contratistas: resultado(c), empresas: resultado(e) } };
}

async function leer(env, id) {
  exigirId(id);
  const actual = await dueno(
    env,
    `SELECT ${CONTRATISTA} FROM contratista WHERE ${POR_ID} LIMIT 1;`,
    { id },
  );
  if (!actual) throw new ErrorDeRegla('no_encontrado');
  return actual;
}

/** Lo que la regla necesita saber de la base, averiguado con la cédula ya en su forma única. */
async function hechos(env, formulario, id, estaGuardado) {
  const cedula = contratista_cedula(texto(formulario.cedula));
  const poseedor = cedula
    ? await dueno(
        env,
        'SELECT VALUE record::id(id) FROM contratista WHERE cedula = $cedula LIMIT 1;',
        {
          cedula,
        },
      )
    : null;
  const empresa = texto(formulario.empresa);
  const existe = empresa
    ? await dueno(env, `SELECT VALUE id FROM empresa WHERE ${POR_ID} LIMIT 1;`, { id: empresa })
    : null;
  // Sólo importa al editar, y sólo con la cédula de antes: no se cambia la cédula de quien
  // está adentro.
  const adentro = estaGuardado
    ? await dueno(env, 'SELECT VALUE id FROM presencia WHERE record::id(id) = $cedula LIMIT 1;', {
        cedula: estaGuardado.cedula,
      })
    : null;
  return {
    cedula_en_uso: poseedor !== null && poseedor !== id,
    empresa_existe: existe !== null,
    esta_adentro: adentro !== null,
  };
}

const evaluar = (id, anterior, borrador, hechosLeidos) =>
  JSON.parse(
    contratista_evaluar(
      JSON.stringify({ id, ahora_ms: Date.now(), anterior, borrador, hechos: hechosLeidos }),
    ),
  );

/** Lo que llega de la página, limpio: el núcleo lo valida. */
const formularioDe = (cuerpo) => ({
  cedula: texto(cuerpo?.cedula),
  nombre: texto(cuerpo?.nombre),
  empresa: texto(cuerpo?.empresa_id),
  tipo_ingreso: texto(cuerpo?.tipo_ingreso),
  fecha_vencimiento_praind: texto(cuerpo?.fecha_vencimiento_praind),
  tiene_acceso: cuerpo?.tiene_acceso !== false,
});

const FILA = (d, empresaNombre) => ({
  id: d.id,
  cedula: d.cedula,
  nombre: d.nombre,
  empresa_id: d.empresa,
  empresa_nombre: empresaNombre,
  tipo_ingreso: d.tipo_ingreso,
  fecha_vencimiento_praind: d.fecha_vencimiento_praind,
  tiene_acceso: d.tiene_acceso,
});

const nombreDeLaEmpresa = async (env, id) =>
  dueno(env, `SELECT VALUE nombre FROM empresa WHERE ${POR_ID} LIMIT 1;`, { id });

const GUARDAR = `type::record('contratista', <uuid>$id) SET cedula = $cedula, nombre = $nombre,
  nombre_busqueda = $nombre_busqueda, empresa = type::record('empresa', <uuid>$empresa),
  tipo_ingreso = $tipo_ingreso, fecha_vencimiento_praind = <datetime>$fecha,
  tiene_acceso = $tiene_acceso`;

const guardado = (d) => ({
  id: d.id,
  cedula: d.cedula,
  nombre: d.nombre,
  nombre_busqueda: d.nombre_busqueda,
  empresa: d.empresa,
  tipo_ingreso: d.tipo_ingreso,
  fecha: `${d.fecha_vencimiento_praind}T00:00:00Z`,
  tiene_acceso: d.tiene_acceso,
});

/** Los problemas del formulario, todos a la vez, para que la página marque cada campo. */
const conProblemas = (problemas) => ({ estado: 422, cuerpo: { problemas } });

/**
 * Evalúa y guarda. Si dos personas registran la misma cédula a la vez, el índice único de
 * la base rechaza a la segunda y se vuelve a preguntar al núcleo, ahora con el hecho.
 */
async function guardar(env, { id, anterior, cuerpo, accion, sentencia }) {
  const borrador = formularioDe(cuerpo);
  const leidos = await hechos(env, borrador, id, anterior);
  const evaluado = evaluar(id, anterior, borrador, leidos);
  if (evaluado.problemas.length > 0) return conProblemas(evaluado.problemas);

  const { datos, cambios } = evaluado;
  if (cambios.length > 0) {
    try {
      await transaccion(
        env,
        `${sentencia} ${GUARDAR}; ${AUDITORIA}`,
        {
          ...guardado(datos),
          entidad: 'contratista',
          accion,
          cambios,
          operador: ADMINISTRADOR_DEL_PANEL,
        },
        INDICES,
      );
    } catch (error) {
      if (!(error instanceof ErrorDeRegla) || error.codigo !== 'contratista_cedula_unica') {
        throw error;
      }
      return conProblemas(
        evaluar(id, anterior, borrador, { ...leidos, cedula_en_uso: true }).problemas,
      );
    }
  }
  return { cuerpo: { contratista: FILA(datos, await nombreDeLaEmpresa(env, datos.empresa)) } };
}

/** Un contratista nuevo. */
export function registrar(env, cuerpo) {
  return aplicar(async () => {
    const respuesta = await guardar(env, {
      id: crypto.randomUUID(),
      anterior: null,
      cuerpo,
      accion: 'alta',
      sentencia: 'CREATE',
    });
    return { estado: 201, ...respuesta };
  });
}

/** Cambia cualquier dato del contratista, incluido quitarle o devolverle el acceso. */
export function editar(env, id, cuerpo) {
  return aplicar(async () => {
    const actual = await leer(env, id);
    const respuesta = await guardar(env, {
      id,
      anterior: { ...actual, empresa: actual.empresa_id },
      cuerpo,
      accion: 'edicion',
      sentencia: 'UPDATE',
    });
    return { estado: 200, ...respuesta };
  });
}

/** Una empresa nueva, desde el formulario del contratista. */
export function crearEmpresa(env, cuerpo) {
  return aplicar(async () => {
    const nombre = texto(cuerpo?.nombre);
    const id = crypto.randomUUID();
    const evaluar = (enUso) => JSON.parse(empresa_evaluar(id, '', nombre, enUso));
    const normalizado = empresa_nombre(nombre);
    const repetida =
      normalizado !== '' &&
      (await dueno(env, 'SELECT VALUE id FROM empresa WHERE nombre = $nombre LIMIT 1;', {
        nombre: normalizado,
      })) !== null;
    const evaluada = evaluar(repetida);
    if (evaluada.problema) return { estado: 422, cuerpo: { problemas: [evaluada.problema] } };

    try {
      await transaccion(
        env,
        `CREATE type::record('empresa', <uuid>$id) SET nombre = $nombre, nombre_busqueda = $nombre_busqueda;
         ${AUDITORIA}`,
        {
          id,
          nombre: evaluada.nombre,
          nombre_busqueda: evaluada.nombre_busqueda,
          entidad: 'empresa',
          accion: 'alta',
          cambios: evaluada.cambios,
          operador: ADMINISTRADOR_DEL_PANEL,
        },
        INDICES,
      );
    } catch (error) {
      if (!(error instanceof ErrorDeRegla) || error.codigo !== 'empresa_nombre_unico') throw error;
      return { estado: 422, cuerpo: { problemas: [evaluar(true).problema] } };
    }
    return { estado: 201, cuerpo: { empresa: { id, nombre: evaluada.nombre } } };
  });
}
