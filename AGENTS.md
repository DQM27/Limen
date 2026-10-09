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

## Estado actual y siguiente paso

El **núcleo** (todo lo que funciona dentro de un equipo, sin pantalla ni nube) está
completo y probado: dominio, aplicación, `infra-memoria`, `infra-surreal`, `infra-plataforma`
y `composicion`. Cubre contratistas, empresas, gafetes, ingresos y salidas (contratista,
proveedor y correo), personal KOF con su gafete provisional, el buscador y las consultas
de lectura. El detalle está en [`docs/reglas.md`](docs/reglas.md) y en la hoja de ruta de
`docs/arquitectura.md` (sección 13).

Lo que sigue, en este orden: **(1)** app de escritorio con Tauri (`apps/escritorio`),
empezando por contratistas: buscar, registrar, entrar y salir; **(2)** el resto de las
pantallas; **(3)** usuarios y sesión (bloque L) y equipos (bloque M); **(4)** la nube
(`infra-nube`: SurrealDB Cloud gratis en `aws-use1` para empezar, sincronización, Worker de
Cloudflare); **(5)** móvil. El almacén clave-valor (configuración del equipo) se define
cuando la pantalla diga qué necesita.

El personal KOF cuenta como "adentro" mientras tenga un gafete provisional sin devolver
(confirmado por el usuario): entregar el gafete es su entrada y devolverlo, su salida. La
pantalla "dentro" junta las cuatro vías en un solo lugar (útil en una emergencia).

## Cosas que ya costaron un tropiezo

- Corre `cargo clippy --all-targets -- -D warnings` **antes** de cada commit, no sólo
  `cargo test`: las pruebas y la batería de contrato también cumplen los lints (aserciones
  con mensaje, sin indexar, funciones de menos de 100 líneas).
- Todo puerto o repositorio nuevo se implementa en `infra-memoria` y en `infra-surreal`,
  y se prueba con la batería de `pruebas-contrato` (que corre contra los dos).
- Lo que debe ser único entre equipos usa una clave natural en la base (`presencia:⟨cédula⟩`,
  `prestamo_gafete:⟨TIPO-NÚMERO⟩`…): la base rechaza al segundo y el caso de uso traduce el
  choque al error de negocio. El nombre del índice o de la tabla va en `infra-surreal/src/error.rs`.
- El buscador decide el orden en el dominio (`busqueda::relevantes`); los adaptadores sólo
  traen candidatos. No poner reglas de búsqueda en las consultas.
