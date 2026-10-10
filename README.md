# Limen

Control de acceso para instalaciones: contratistas, proveedores, ingresos por correo y
personal KOF. Sucesor de Lattis, rediseñado desde cero con arquitectura hexagonal.

> *Limen*: "umbral" en latín, la línea que se cruza al entrar.

## Estructura

```text
crates/
  dominio/           reglas de negocio puras (compila también a WebAssembly)
  aplicacion/        casos de uso, puertos (traits) y modelo de errores
  infra-memoria/     implementación en memoria de los puertos, para pruebas
  infra-surreal/     SurrealDB embebido (SurrealKV en disco, kv-mem en pruebas)
  infra-plataforma/  reloj confiable (NTP), IDs UUID v7 y claves con Argon2id
  composicion/       raíz de composición: arma la aplicación con sus adaptadores
  pruebas-contrato/  batería que corre contra todos los adaptadores
apps/
  escritorio/        Tauri 2 + Angular 22 (Angular Material, AG Grid)
    comandos/        la lógica de los comandos, sin Tauri (se prueba en memoria)
    src-tauri/       el cascarón de Tauri
```

Faltan la nube (`infra-nube`) y la app móvil. El estado y el siguiente paso están en
[`docs/continuar.md`](docs/continuar.md).

## Desarrollo

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all -- -D warnings
cargo test
```

En `apps/escritorio`: `npx prettier --check "src/**/*.{ts,html,scss}"`,
`npx ng test --no-watch`, `npm run build` y, para abrir la app, `npx tauri dev`.

- [`docs/arquitectura.md`](docs/arquitectura.md): capas, patrones (hexagonal, Unit of Work,
  raíz de composición), base de datos, sincronización, pruebas y recetas.
- [`docs/reglas.md`](docs/reglas.md): catálogo de reglas de negocio acordadas.
- [`docs/operacion-sin-conexion.md`](docs/operacion-sin-conexion.md): cómo trabaja la
  portería sin red (nunca se bloquea) y cómo se resuelven los choques al sincronizar.
- [`docs/continuar.md`](docs/continuar.md): dónde quedó el trabajo y qué sigue.
