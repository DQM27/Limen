// API del panel de equipos. La página es Angular (web/), servida como archivos estáticos.
// Habla con la nube con credenciales que sólo viven en el Worker.
// Acceso con clave simple (Basic) mientras no se ponga Cloudflare Access.

import { consultar, resultado, LECTURA } from './nube.js';
import { acceso } from './acceso.js';
import { estado_de_conexion_del_equipo } from './dominio.js';
import { enVivo } from './en_vivo.js';
import { canjear, listar, registrar, retirar, unidades } from './vinculacion.js';
import {
  crearEmpresa,
  editar as editarContratista,
  listar as listarContratistas,
  registrar as registrarContratista,
} from './contratistas.js';
import {
  crear as crearUsuario,
  editar as editarUsuario,
  listar as listarUsuarios,
  restablecerClave,
} from './usuarios.js';

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
    // Cómo está conectado lo decide el núcleo (en línea, sin señal o desconectado).
    estado: estado_de_conexion_del_equipo(Boolean(f.en_linea), Date.parse(f.visto), ahora),
  }));
}

const json = ({ estado, cuerpo }) => Response.json(cuerpo, { status: estado });

/** Todo el panel va por HTTPS: la clave y el código no deben viajar sin cifrar. */
const conSeguridad = (respuesta) => {
  const cabeceras = new Headers(respuesta.headers);
  cabeceras.set('Strict-Transport-Security', 'max-age=31536000; includeSubDomains');
  cabeceras.set('X-Content-Type-Options', 'nosniff');
  cabeceras.set('Referrer-Policy', 'no-referrer');
  return new Response(respuesta.body, {
    status: respuesta.status,
    headers: cabeceras,
  });
};

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.protocol === 'http:') {
      // Antes de pedir ninguna clave: así ni siquiera se envía por un camino sin cifrar.
      url.protocol = 'https:';
      return Response.redirect(url.toString(), 301);
    }
    return conSeguridad(await atender(request, env));
  },
};

async function atender(request, env) {
  const { pathname } = new URL(request.url);
  try {
    // El único camino sin clave del panel: el equipo canjea su código (el código es su
    // credencial, de un solo uso y de 15 minutos).
    if (pathname === '/api/vincular' && request.method === 'POST') {
      return json(await canjear(env, await request.json()));
    }
    // Cerrar sesión: el navegador guarda la clave de la ventana de acceso y sólo la suelta
    // cuando el servidor la rechaza. Esta ruta siempre responde 401 con una página de salida.
    if (pathname === '/salir') {
      return new Response(
        '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">' +
          '<title>Limen · sesión cerrada</title><body style="font:16px system-ui;margin:3rem auto;max-width:28rem;padding:0 1rem">' +
          '<h1>Sesión cerrada</h1><p><a href="/">Volver a entrar</a></p>',
        {
          status: 401,
          headers: {
            'Content-Type': 'text/html; charset=utf-8',
            'WWW-Authenticate': 'Basic realm="Limen"',
            'Cache-Control': 'no-store',
          },
        },
      );
    }
    // El otro camino sin clave del panel: el equipo prueba quién es con su firma y recibe su
    // acceso a la nube.
    if (pathname === '/api/token' && request.method === 'POST') {
      return json(await acceso(env, await request.json()));
    }
    if (!autorizado(request, env)) {
      return new Response('Acceso restringido', {
        status: 401,
        headers: { 'WWW-Authenticate': 'Basic realm="Limen"' },
      });
    }
    if (pathname === '/api/equipos') return Response.json({ equipos: await equipos(env) });
    // El token con que la página se conecta en vivo a la base (sólo lectura, una hora).
    if (pathname === '/api/en-vivo' && request.method === 'POST') return json(await enVivo(env));
    if (pathname === '/api/unidades') return Response.json({ unidades: await unidades(env) });
    if (pathname === '/api/dispositivos' && request.method === 'GET') {
      return Response.json({ dispositivos: await listar(env) });
    }
    if (pathname === '/api/dispositivos' && request.method === 'POST') {
      return json(await registrar(env, await request.json()));
    }
    const retiro = pathname.match(/^\/api\/dispositivos\/([0-9a-f-]{36})\/retirar$/);
    if (retiro && request.method === 'POST') return json(await retirar(env, retiro[1]));
    // Usuarios: lo que se decide lo decide el núcleo; aquí sólo se enruta.
    const cuerpoJson = () => request.json().catch(() => ({}));
    if (pathname === '/api/usuarios' && request.method === 'GET')
      return json(await listarUsuarios(env));
    if (pathname === '/api/usuarios' && request.method === 'POST') {
      return json(await crearUsuario(env, await cuerpoJson()));
    }
    const usuario = pathname.match(/^\/api\/usuarios\/([^/]+)$/);
    if (usuario && request.method === 'PATCH') {
      return json(await editarUsuario(env, usuario[1], await cuerpoJson()));
    }
    const clave = pathname.match(/^\/api\/usuarios\/([^/]+)\/clave$/);
    if (clave && request.method === 'POST') return json(await restablecerClave(env, clave[1]));
    // Contratistas y sus empresas: igual, el núcleo decide.
    if (pathname === '/api/contratistas' && request.method === 'GET') {
      return json(await listarContratistas(env));
    }
    if (pathname === '/api/contratistas' && request.method === 'POST') {
      return json(await registrarContratista(env, await cuerpoJson()));
    }
    const contratista = pathname.match(/^\/api\/contratistas\/([^/]+)$/);
    if (contratista && request.method === 'PATCH') {
      return json(await editarContratista(env, contratista[1], await cuerpoJson()));
    }
    if (pathname === '/api/empresas' && request.method === 'POST') {
      return json(await crearEmpresa(env, await cuerpoJson()));
    }
    if (pathname.startsWith('/api/')) return Response.json({ error: 'no_existe' }, { status: 404 });
    return env.ASSETS.fetch(request);
  } catch (error) {
    // Queda en los registros del Worker (`wrangler tail`); al navegador sólo llega el código.
    console.error('falla del panel:', pathname, error);
    return Response.json({ error: 'nube' }, { status: 502 });
  }
}
