# Para continuar donde se quedó

Lee primero [`AGENTS.md`](../AGENTS.md) (reglas del dueño y de arquitectura) y, si hace
falta el detalle, [`arquitectura.md`](arquitectura.md), [`reglas.md`](reglas.md) y
[`importador-lattis.md`](importador-lattis.md).

## Dónde quedó todo

- **Núcleo** terminado: dominio, aplicación, persistencia (memoria y SurrealDB),
  composición, buscador y consultas de lectura. Además, el historial de ingresos por
  rango de fechas y los accesos rápidos de fecha (`dominio/rango_fechas.rs`).
- **App de escritorio** (Tauri 2 + Angular 22 + Angular Material + AG Grid Community):
  - `apps/escritorio/comandos`: la lógica de los comandos (JSON, un solo error para la
    interfaz, operador provisional). Se prueba con la base en memoria.
  - `apps/escritorio/src-tauri`: cascarón de Tauri, sin lógica ni pruebas propias (un
    ejecutable de pruebas que enlaza Tauri no arranca en Windows).
  - Frontend: barra lateral, `compartido/tabla` (la grilla base de toda la app, con el
    diseño unificado) y la pantalla de Contratistas.
- **Importador de Lattis** (`herramientas/importador-lattis`): carga un volcado SQL en la
  base de Limen. Los datos reales viven en `datos-privados/` (ignorada por git) y **nunca
  se suben**: el repositorio es público.

## Ramas y PR (nada va a `main` sin PR y CI en verde)

Las ramas están apiladas, en este orden: `fix/pruebas-en-windows` (PR #1) →
`ci/endurecer` (PR #2) → `feat/app-escritorio` → `feat/contratistas-grilla` (la de
trabajo actual). `feat/importador-lattis` sale de `feat/app-escritorio`. **Primero hay
que fusionar #1 y #2** (el dueño hace el merge); después se rebasa el resto sobre `main`
y se abren sus PR. Falta proteger `main` en GitHub (exigir PR y el resultado `CI verde`).

## Lo que sigue

1. **Historial, segunda mitad.** Conectar `ListarHistorial` y `AtajosDeFecha` en
   `composicion`, en `apps/escritorio/comandos` (DTO + comandos `listar_historial` y
   `atajos_de_fecha`) y en `src-tauri`. Luego la pantalla en Angular sobre la grilla base,
   con un selector de rango de fechas (botón con la etiqueta corta, panel con los accesos
   rápidos que da el núcleo y fechas Desde/Hasta; el modelo es el de Lattis) y las
   exportaciones: CSV, Excel y PDF (necesitan un comando de Rust que guarde el archivo y
   el plugin de diálogo de Tauri).
2. **Formulario de "Nuevo contratista"** (diálogo de Material, con el error del núcleo
   junto a cada campo por su `codigo`) y el botón en la fila de herramientas de la grilla.
3. **CI para Node y Tauri**: un trabajo en Windows (instalar, lint, pruebas, build de
   Angular, clippy del cascarón) y excluir `limen-escritorio` de los trabajos de Linux, que
   no tienen las bibliotecas de Tauri. Activar CodeQL cuando haya JavaScript que analizar.
4. Otras pantallas de la barra lateral (Dentro, Proveedores, Correo, Personal KOF,
   Gafetes), el inicio de sesión real (bloque L) y la nube, en ese orden.

## Cómo trabajar en esta máquina

- No hay Visual Studio: compilar con el toolchain GNU. En PowerShell:
  `$env:RUSTUP_TOOLCHAIN='1.99.0-x86_64-pc-windows-gnu'` (no se toca
  `rust-toolchain.toml`, que usa la CI en Linux). El `link` de Git Bash rompe MSVC.
- Verificar antes de cada commit: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --all -- -D warnings`, `cargo test --all`; y en
  `apps/escritorio`: `npx prettier --check "src/**/*.{ts,html,scss}"`, `npx ng test
  --no-watch` y `npm run build`.
- La app en desarrollo: `npx tauri dev` dentro de `apps/escritorio`. Los paquetes npm de
  Tauri deben coincidir en versión menor con el crate de Rust (hoy 2.11).
- Cerrar la app antes de importar datos: la base queda tomada mientras esté abierta.
- Si un cambio de Rust deja el código a medias, `tauri dev` se detiene solo (vigila los
  crates): se relanza al terminar.

## Reglas del dueño que no se negocian

- Comunicarse siempre **en español**.
- **Nunca** usar el término prohibido para el puesto de control (ver `AGENTS.md`); decir
  "puesto de control", "portería" o "punto de acceso".
- Cada cambio exitoso: commit bien documentado **y subido**. Los commits van sólo a nombre
  de Daniel Quintana, sin líneas `Co-Authored-By` ni atribuciones a asistentes.
- La interfaz **no decide nada**: sólo muestra y envía; toda regla vive en el dominio.
- Corre `clippy --all-targets` **antes** de cada commit, no sólo las pruebas.
- Después de subir, confirma que el CI de GitHub quedó en verde.
- **Nada se mergea a `main` sin un PR con su CI**; los PR se abren cuando el dueño lo
  indica o tras fusionar los pendientes.
- Los datos reales del cliente nunca entran al repositorio.
