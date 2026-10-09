# Para continuar donde se quedó

Lee primero [`AGENTS.md`](../AGENTS.md) (reglas del dueño y de arquitectura) y, si hace
falta el detalle, [`arquitectura.md`](arquitectura.md) y [`reglas.md`](reglas.md).

## Dónde quedó todo

El **núcleo** de Limen está terminado y subido a `main`: dominio, aplicación, los dos
adaptadores de persistencia (memoria y SurrealDB), la composición, el buscador y las
consultas de lectura. 373 pruebas en verde, clippy limpio y CI en verde.

Funciona de punta a punta, sin pantalla: empresas, contratistas, gafetes, ingreso y
salida por las cuatro vías (contratista, proveedor, correo, personal KOF), la lista de
"dentro", el historial de cambios y el PRAIND por vencer.

Verifica que todo está bien antes de empezar (en la raíz del repositorio):

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all -- -D warnings
cargo test --all
```

## Lo que sigue: la app de escritorio (Tauri + React)

La sesión anterior corría en la nube y no podía abrir ventanas. **Tú sí puedes**: aquí
se compila y se corre con `cargo tauri dev`.

Orden acordado con el dueño:

1. **Librería con la lógica de los comandos** (un crate en `apps/escritorio/`): recibe
   la `AplicacionLimen` de `crates/composicion`, convierte los datos a JSON (el dominio no
   es serializable a propósito) y traduce los errores con `ErrorCaso::para_interfaz`.
   Se prueba con la base en memoria, sin necesitar Tauri.
2. **Esqueleto de la app:** cascarón de Tauri mínimo (sólo registra los comandos y abre la
   base con `AplicacionLimen::abrir`) y el frontend en React + TypeScript + Vite + Vitest.
   Las dependencias van hacia adentro: `apps → composicion → …` (sección 3 de la
   arquitectura). Ningún `if` de negocio en la interfaz.
3. **Primera pantalla: "Dentro"**, con las cuatro vías juntas (útil en una emergencia) y
   la salida desde la misma lista; y el buscador de contratistas.
4. **Registrar un contratista y su entrada.**

Agrega también al CI (`.github/workflows/ci.yml`) un trabajo que instale las bibliotecas
de Tauri y compile el cascarón, para que se verifique en cada subida.

### Decisión provisional que hay que respetar

Los usuarios se crean en el panel (nube), que todavía no existe (bloque L). Mientras
tanto la app pide **un operador provisional en la primera ejecución** y lo guarda
localmente. Márcalo como temporal en el código y en `reglas.md`.

## Huecos conocidos (se resuelven al llegar a su pantalla)

- Listados con filtros y páginas: la grilla de contratistas y el historial de ingresos
  todavía no existen como consulta.
- Almacén clave-valor para la configuración del equipo: se define cuando una pantalla
  diga qué necesita.
- La nube, la sincronización y el aviso de "sin conexión" van después de las pantallas
  (lee `operacion-sin-conexion.md` antes de tocarlas).

## Reglas del dueño que no se negocian

- Comunicarse siempre **en español**.
- **Nunca** usar el término prohibido para el puesto de control (ver `AGENTS.md`); decir
  "puesto de control", "portería" o "punto de acceso".
- Cada cambio exitoso: commit bien documentado **y subido**. Los commits van sólo a nombre
  de Daniel Quintana, sin líneas `Co-Authored-By` ni atribuciones a asistentes.
- Corre `clippy --all-targets` **antes** de cada commit, no sólo las pruebas.
- Después de subir, confirma que el CI de GitHub quedó en verde.
- No crear pull requests si no se piden.
