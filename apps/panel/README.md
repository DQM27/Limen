# Panel de dispositivos

Worker de Cloudflare con una página mínima: equipos de la tabla `conexion` (la
escribe la app de escritorio, `crates/infra-nube/src/presencia.rs`) y su estado:
**en línea**, **caído** (más de 10 minutos sin avisar) o **fuera** (se despidió).

```text
cd apps/panel
wrangler secret put SURREAL_URL       # https://…aws-use1.surreal.cloud
wrangler secret put SURREAL_USUARIO
wrangler secret put SURREAL_CLAVE
wrangler secret put PANEL_CLAVE       # clave para entrar al panel (usuario: cualquiera)
wrangler dev                          # probar en local
wrangler deploy
```

Subdominio propio: añadir en `wrangler.toml` una ruta `panel.<dominio>/*` con la zona
del dominio en Cloudflare. La página consulta cada 15 s sólo con la pestaña visible.
