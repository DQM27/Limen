# Notas del proyecto

## Preferencias del usuario

- **NUNCA usar el término prohibido para el puesto de control** en conversación, código,
  comentarios ni documentación. Para referirse al puesto de control de acceso usar
  términos como "puesto de control", "portería" o "punto de acceso".
- Comunicarse siempre en español.
- Siempre que se complete un cambio exitoso, hacer un commit bien documentado y subirlo.
- **Todos los commits van sólo a nombre de `Daniel Quintana <daniel.bleach1@gmail.com>`**,
  como autor y como committer (`git config user.name`/`user.email` antes del primer
  commit). Sin líneas `Co-Authored-By`, `Claude-Session` ni ninguna otra atribución a
  asistentes en commits ni en pull requests.

## Reglas de arquitectura

La guía completa está en [`docs/arquitectura.md`](docs/arquitectura.md): léela antes de
empezar a trabajar. Resumen:

- Arquitectura hexagonal. Dependencias sólo hacia adentro:
  `apps → composicion → infra-* → aplicacion → dominio`.
- **Toda regla de negocio vive en `crates/dominio`**, sin excepción. Si una regla
  necesita datos (cédula repetida, "está adentro"), el caso de uso los busca y se los
  entrega al dominio, que decide. Los casos de uso orquestan; no deciden.
- El dominio es puro: sin base de datos, red, reloj del sistema ni generación
  aleatoria. Recibe la fecha de hoy y los IDs. `tests/arquitectura.rs` lo vigila.
- No hay un crate de "reglas" aparte para WebAssembly: el dominio ya compila a WASM.
- Sin ORM. Las consultas de SurrealDB se escriben a la vista.
- El catálogo de reglas acordadas está en `docs/reglas.md`; cualquier cambio de regla
  se refleja ahí.

## Verificación antes de cada commit

La versión de Rust está fija en `rust-toolchain.toml`; `cargo` la usa sola. Después de
subir, revisar que la CI de GitHub quede en verde.

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```
