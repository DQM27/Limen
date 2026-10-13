// Panel mínimo de dispositivos: lista los equipos y si están en línea.
// Lee la tabla `conexion` de la nube con credenciales que sólo viven en el Worker.
// Acceso con clave simple (Basic) mientras no se ponga Cloudflare Access.

const NAMESPACE = 'limen';
const BASE = 'principal';
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

// SurrealDB no acepta a un usuario de base de datos con Basic auth: se entra por
// /signin y se usa el token (dura 1 h). Se guarda mientras viva el Worker.
let sesion = { token: '', hasta: 0 };

async function token(env) {
  if (sesion.token && Date.now() < sesion.hasta) return sesion.token;
  const respuesta = await fetch(`${env.SURREAL_URL}/signin`, {
    method: 'POST',
    headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: JSON.stringify({
      ns: NAMESPACE,
      db: BASE,
      user: env.SURREAL_USUARIO,
      pass: env.SURREAL_CLAVE,
    }),
  });
  const cuerpo = await respuesta.json();
  if (!cuerpo.token) throw new Error('signin');
  sesion = { token: cuerpo.token, hasta: Date.now() + 50 * 60 * 1000 };
  return sesion.token;
}

async function equipos(env) {
  const respuesta = await fetch(`${env.SURREAL_URL}/sql`, {
    method: 'POST',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'text/plain',
      'Surreal-NS': NAMESPACE,
      'Surreal-DB': BASE,
      Authorization: 'Bearer ' + (await token(env)),
    },
    body: 'SELECT equipo, unidad, en_linea, iniciada, visto FROM conexion ORDER BY unidad, equipo;',
  });
  const [resultado] = await respuesta.json();
  // Si todavía ningún equipo anunció su conexión, la tabla no existe: no hay equipos.
  if (resultado?.status === 'ERR' && resultado.kind === 'NotFound') return [];
  if (resultado?.status !== 'OK') throw new Error('consulta');
  const ahora = Date.now();
  return resultado.result.map((f) => ({
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

export default {
  async fetch(request, env) {
    if (!autorizado(request, env)) {
      return new Response('Acceso restringido', {
        status: 401,
        headers: { 'WWW-Authenticate': 'Basic realm="Limen"' },
      });
    }
    const { pathname } = new URL(request.url);
    if (pathname === '/api/equipos') {
      try {
        return Response.json({ equipos: await equipos(env) });
      } catch {
        return Response.json({ error: 'nube' }, { status: 502 });
      }
    }
    return new Response(PAGINA, { headers: { 'Content-Type': 'text/html; charset=utf-8' } });
  },
};
