// API del panel de equipos. La página es Angular (web/), servida como archivos estáticos.
// Habla con la nube con credenciales que sólo viven en el Worker.
// Acceso con clave simple (Basic) mientras no se ponga Cloudflare Access.

import { consultar, resultado, LECTURA } from './nube.js';
import { canjear, listar, registrar, retirar } from './vinculacion.js';
// Un equipo que lleva más de dos latidos (5 min c/u) sin avisar se da por caído.
const CAIDO_DESPUES_DE_MS = 10 * 60 * 1000;

function autorizado(request, env) {
  const cabecera = request.headers.get('Authorization') ?? '';
  if (!cabecera.startsWith('Basic ') || !env.PANEL_CLAVE) return false;
  const [, clave = ''] = atob(cabecera.slice(6)).split(/:(.*)/s);
  return clave === env.PANEL_CLAVE;
}

async function equipos(env) {
  const [s] = await consultar(
    env,
    LECTURA,
    'SELECT equipo, unidad, en_linea, iniciada, visto FROM conexion ORDER BY unidad, equipo;',
  );
  // Si todavía ningún equipo anunció su conexión, la tabla no existe: no hay equipos.
  const ahora = Date.now();
  return resultado(s).map((f) => ({
    equipo: f.equipo,
    unidad: f.unidad,
    iniciada: f.iniciada,
    visto: f.visto,
    estado: !f.en_linea
      ? 'fuera'
      : ahora - Date.parse(f.visto) > CAIDO_DESPUES_DE_MS
        ? 'caido'
        : 'en_linea',
  }));
}

const json = ({ estado, cuerpo }) => Response.json(cuerpo, { status: estado });

export default {
  async fetch(request, env) {
    const { pathname } = new URL(request.url);
    try {
      // El único camino sin clave del panel: el equipo canjea su código (el código es su
      // credencial, de un solo uso y de 15 minutos).
      if (pathname === '/api/vincular' && request.method === 'POST') {
        return json(await canjear(env, await request.json()));
      }
      if (!autorizado(request, env)) {
        return new Response('Acceso restringido', {
          status: 401,
          headers: { 'WWW-Authenticate': 'Basic realm="Limen"' },
        });
      }
      if (pathname === '/api/equipos') return Response.json({ equipos: await equipos(env) });
      if (pathname === '/api/dispositivos' && request.method === 'GET') {
        return Response.json({ dispositivos: await listar(env) });
      }
      if (pathname === '/api/dispositivos' && request.method === 'POST') {
        return json(await registrar(env, await request.json()));
      }
      const retiro = pathname.match(/^\/api\/dispositivos\/([0-9a-f-]{36})\/retirar$/);
      if (retiro && request.method === 'POST') return json(await retirar(env, retiro[1]));
      if (pathname.startsWith('/api/')) return Response.json({ error: 'no_existe' }, { status: 404 });
      return env.ASSETS.fetch(request);
    } catch {
      return Response.json({ error: 'nube' }, { status: 502 });
    }
  },
};
