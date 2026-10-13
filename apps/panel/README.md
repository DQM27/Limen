# Panel de equipos

Worker de Cloudflare con una página Angular (`web/`): registrar y retirar equipos con su código de vinculación, y ver los conectados.
Antes era sólo la lista: equipos de la tabla `conexion` (la
escribe la app de escritorio, `crates/infra-nube/src/presencia.rs`) y su estado:
**en línea**, **caído** (más de 10 minutos sin avisar) o **fuera** (se despidió).

```text
cd apps/panel
wrangler secret put SURREAL_URL       # https://…aws-use1.surreal.cloud
wrangler secret put SURREAL_USUARIO   # panel (sólo lectura)
wrangler secret put SURREAL_CLAVE
wrangler secret put SURREAL_ADMIN_USUARIO   # panel_admin (escribe: registrar y retirar equipos)
wrangler secret put SURREAL_ADMIN_CLAVE
wrangler secret put PANEL_CLAVE       # clave para entrar al panel (usuario: cualquiera)
npm run publicar                      # construye la página (Angular, web/) y despliega
```

En Windows, si PowerShell no deja correr `wrangler`, usar `wrangler.cmd`.

Subdominio propio: añadir en `wrangler.toml` una ruta `panel.<dominio>/*` con la zona
del dominio en Cloudflare. La página consulta cada 15 s sólo con la pestaña visible.

## El dominio dentro del Worker

Las reglas de vinculación (código, vigencia, un solo uso) son de `crates/dominio`; el Worker
las usa compiladas a WebAssembly (`crates/dominio-wasm`, resultado en `src/dominio/`).
Si cambian las reglas, desde la **raíz del repositorio**:

```text
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked   # una vez
npm --prefix apps/panel run dominio
```

y se sube el resultado. Para registrar equipos hace falta además un usuario de base de datos
que pueda escribir (`SURREAL_ADMIN_USUARIO` y `SURREAL_ADMIN_CLAVE`, `wrangler secret put`).

## API

| Camino | Quién | Qué hace |
|---|---|---|
| `GET /api/equipos` | panel | equipos conectados (`conexion`) |
| `GET /api/dispositivos` | panel | equipos registrados y su estado |
| `POST /api/dispositivos` | panel | registra un equipo y emite su código (`nombre`, `unidad`, `tipo`) |
| `POST /api/dispositivos/:id/retirar` | panel | retira el equipo y anula su código pendiente |
| `POST /api/vincular` | el equipo | canjea el código y deja su clave pública (`codigo`, `clave_publica`) |
