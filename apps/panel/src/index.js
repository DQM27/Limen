// Panel mínimo de dispositivos: lista los equipos y si están en línea.
// Lee la tabla `conexion` de la nube con credenciales que sólo viven en el Worker.
// Acceso con clave simple (Basic) mientras no se ponga Cloudflare Access.

import { consultar, resultado, LECTURA } from './nube.js';
import { canjear, listar, registrar, retirar } from './vinculacion.js';
// Un equipo que lleva más de dos latidos (5 min c/u) sin avisar se da por caído.
const CAIDO_DESPUES_DE_MS = 10 * 60 * 1000;

const PAGINA = `<!doctype html><html lang="es"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Limen · dispositivos</title>
<style>body{font:15px system-ui;margin:1.5rem auto;max-width:48rem;padding:0 1rem}
table{width:100%;border-collapse:collapse}td,th{padding:.4rem .6rem;border-bottom:1px solid #8884;text-align:left}
.en_linea{color:#1a7f37}.caido{color:#cf222e}.fuera{color:#888}</style>
<h1>Dispositivos</h1><p id="r">Cargando…</p>
<table><thead><tr><th>Equipo<th>Unidad<th>Estado<th>Visto<th>Conectado desde</thead><tbody id="t"></tbody></table>
<script>
const fecha=(v)=>v?new Date(v).toLocaleString('es-CR'):'';
async function cargar(){
  if(document.hidden)return;
  try{
    const r=await fetch('/api/equipos');const d=await r.json();
    document.getElementById('t').innerHTML=d.equipos.map(e=>
      '<tr><td>'+e.equipo.slice(0,8)+'<td>'+e.unidad+'<td class="'+e.estado+'">'+e.estado.replace('_',' ')+
      '<td>'+fecha(e.visto)+'<td>'+fecha(e.iniciada)).join('');
    document.getElementById('r').textContent=d.equipos.length+' equipos · actualizado '+fecha(Date.now());
  }catch{document.getElementById('r').textContent='Sin respuesta de la nube';}
}
cargar();setInterval(cargar,15000);document.addEventListener('visibilitychange',cargar);
</script></html>`;

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
      return new Response(PAGINA, { headers: { 'Content-Type': 'text/html; charset=utf-8' } });
    } catch {
      return Response.json({ error: 'nube' }, { status: 502 });
    }
  },
};
