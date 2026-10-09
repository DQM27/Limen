# Limen

Control de acceso para instalaciones: contratistas, proveedores, ingresos por correo y
personal KOF. Sucesor de Lattis, rediseñado desde cero con arquitectura hexagonal.

> *Limen*: "umbral" en latín, la línea que se cruza al entrar.

## Estructura

```text
crates/
  dominio/      reglas de negocio puras (compila también a WebAssembly)
```

Próximas capas, en orden: `aplicacion` (casos de uso y puertos), `infra-surreal`
(SurrealDB embebido), `infra-memoria` (dobles para pruebas), `composicion` y las apps.

## Desarrollo

```text
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

Las reglas de negocio acordadas están en [`docs/reglas.md`](docs/reglas.md).
