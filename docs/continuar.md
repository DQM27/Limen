# Para continuar donde se quedó

Lee primero [`AGENTS.md`](../AGENTS.md) (reglas del dueño y de arquitectura) y, si hace
falta el detalle, [`arquitectura.md`](arquitectura.md), [`reglas.md`](reglas.md) y
[`operacion-sin-conexion.md`](operacion-sin-conexion.md).

## Dónde quedó todo

- **Núcleo** terminado: dominio, aplicación, persistencia (memoria y SurrealDB),
  composición, buscador y consultas de lectura. Además, el historial de ingresos por
  rango de fechas y los accesos rápidos de fecha (`dominio/rango_fechas.rs`).
- **Responsable de cada marca** (regla E12): "dentro" y el historial traen, para la
  entrada y para la salida por separado, el instante (fecha y hora juntas, UTC) y quién
  la registró (`entrada_por`/`salida_por`: ID y nombre del usuario; nombre `null` si no
  está en el equipo). La pantalla muestra fecha y hora en columnas aparte, en hora de
  Costa Rica.
- En el código se dice **clave**, no "contraseña" (tipos, comandos, campos del esquema
  `usuario.clave` y `debe_cambiar_clave`, códigos de error como `clave_corta`).
- **Hechos inmutables** (regla E11, sección 9.1 de `arquitectura.md`): cada entrada y
  salida, por las cuatro vías, queda en la tabla `hecho` en la misma transacción que el
  estado. La base no deja editar ni borrar `hecho` ni `auditoria` (`READONLY` y un
  `DEFINE EVENT`). Es la base de la sincronización.
- **Usuarios e inicio de sesión** (bloque L, reglas L1–L3 y L6–L8): cédula y
  clave, Argon2id (`infra-plataforma`), bloqueo tras 5 fallos y clave temporal.
  **Los usuarios son globales y vienen de la nube**: el equipo sólo inicia sesión,
  cambia la propia clave (actual y nueva) y cierra la sesión; no crea ni administra
  usuarios. **La app arranca sin sesión** y ningún comando hace nada hasta entrar.
- **Semilla de desarrollo** (`composicion/src/semilla.rs`, sólo en compilación de
  depuración): si la base no tiene usuarios, al abrirla se siembran `100000001`
  (OPERADOR UNO) y `100000002` (OPERADOR DOS), los dos con la clave `desarrollo`. La
  versión para producción no trae la semilla; ahí los usuarios llegarán de la nube.
- **Reloj confiable** (regla E5): la hora sale de NTP (`time.windows.com`, luego
  `pool.ntp.org`) anclada al reloj monotónico del proceso; se vuelve a medir cada hora en
  segundo plano y el último desfase se guarda en `reloj-desfase.txt` (carpeta de datos de
  la app). El reloj ya no bloquea: un movimiento con hora dudosa se registra y su hecho
  queda marcado (`hora_confiable = false`, con la `hora_equipo` cruda). **Pendiente de
  probar en la PC**: que la red de la planta deje salir NTP (UDP 123); si no, todo queda
  "sin comprobar" hasta que exista la nube, que dará la hora.
- **Importador de Lattis**: vive todavía en la rama `feat/importador-lattis` (no en
  `main`), con su documento `docs/importador-lattis.md`. Carga un volcado SQL en la base
  de Limen. Los datos reales viven en `datos-privados/` (ignorada por git) y **nunca se
  suben**: el repositorio es público.

### App de escritorio (Tauri 2 + Angular 22 + Angular Material + AG Grid Community)

- `apps/escritorio/comandos`: la lógica de los comandos (JSON, un solo error para la
  interfaz, la sesión del equipo). Se prueba con la base en memoria.
- `apps/escritorio/src-tauri`: cascarón de Tauri, sin lógica ni pruebas propias (un
  ejecutable de pruebas que enlaza Tauri no arranca en Windows).
- **Ventana y entrada:** una sola ventana. Nace oculta, pequeña (380×480), sin marco ni
  sombra; muestra el splash (estático en `index.html`, luego `Splash`, mínimo 3 s:
  `DURACION_MINIMA_SPLASH_MS`) y la pantalla de entrada (cédula y clave, "Recordar
  usuario" sólo guarda la cédula; con clave temporal pide cambiarla). Al entrar pasa a
  maximizada y con marco (`modo_ventana`). Un `sin_sesion` de cualquier comando vuelve a
  la entrada.
- **Tema:** Tokyo Night (Night en oscuro, Day en claro, según el sistema) sobre los
  tokens de Material: `src/_tokyo-night.scss`. Los diálogos comparten estilos
  (`compartido/_dialogo.scss`).
- **Avisos** (`compartido/avisos.ts`): al estilo de las notificaciones de macOS, arriba a
  la derecha, se apilan y se van solos. Usar `Avisos.exito/error/info(mensaje, título?)`,
  no `MatSnackBar`.
- **Pantallas** (barra lateral, ya todas activas):
  - **Dentro:** quién está adentro por las cuatro vías (`dentro/lista-dentro.ts`, pieza
    reutilizable con la vía como entrada), salida por fila y masiva con confirmación,
    filtro con/sin gafete y resaltado de quien lleva más de 12 horas (regla E13).
  - **Contratistas:** grilla, "Nuevo contratista" (el mismo formulario edita con doble
    clic), "Registrar ingreso" y botón "Ingreso" por fila. El formulario valida **todo a
    la vez** (`validar_contratista`, regla B5b) y la empresa sólo se elige: crearla es el
    botón «+» con su propio modal.
  - **Historial:** entradas y salidas de un rango (`historial/historial-pagina.ts`). Botón
    con los atajos de fecha del núcleo (abre con los últimos 6 meses), fechas Desde/Hasta
    a mano, aviso si el rango se trunca y el error del rango junto a su fecha. Las
    exportaciones (CSV, Excel, PDF) quedan para el final, a pedido del dueño.
  - **Proveedores, Correo y KOF:** cada una es la lista de Dentro filtrada por su vía,
    con su salida y su botón de ingreso (en KOF, "Entregar gafete", "Nuevo personal" y
    "Personal", que corrige el nombre o desactiva y activa; el código no se edita). Piezas
    compartidas: `SelectorEmpresa` (también en el formulario de contratista),
    `CamposDeIngreso`, `DialogoEmpresa`. Todos los formularios de ingreso informan **todos**
    sus problemas a la vez (reglas B5b, H4, I3 y K5: `validar_*`).
  - **Atajos de teclado** (directiva `compartido/atajo.ts`): `F2` registra el ingreso (o
    entrega el gafete en KOF), `Ctrl+N` crea (contratista, personal), `/` busca en la
    tabla y `F4` lleva al campo de **salida por gafete** de las listas de Dentro (número y
    Enter). Si el mismo número está en uso en dos vías no se adivina: pide usar el botón
    de la fila. Con un modal abierto los atajos de atrás no valen.
  - **Gafetes:** por tipo, con "asignado a" (regla F7), crear por rango y gestionar
    (perdido con su último portador, pagado, apareció, de baja).
- Lo que se escribe se ve como quedará al guardarse (nombre y empresa en mayúsculas,
  placa): la forma la da el dominio (`nombre_persona_al_escribir`, etc.).

## Ramas y PR (nada va a `main` sin PR y CI en verde)

El trabajo está en la rama **`feat/login`** (desde `main`), **sin PR todavía: el dueño
avisa cuándo abrirlo**. Trae: entrada y splash, tema, formulario de contratista,
Gafetes, Dentro, avisos, ingresos de contratista, proveedor, correo y KOF.
`feat/importador-lattis` sigue aparte: necesita traer `main` y generar los hechos (con la
hora sellada) de los ingresos importados antes de su PR. `fix/pruebas-en-windows` tiene 3
commits en el remoto que no están en `main`: revisarlos o descartarlos.

## Decidido y en espera

- **Cifrado en reposo:** se espera a que SurrealKV publique su cifrado (TDE, ya en su
  rama principal) y a que SurrealDB lo conecte; la clave, como en Lattis (DPAPI en
  Windows, Keystore en Android). Detalle en `arquitectura.md`, sección 8.2. Revisar las
  versiones nuevas de `surrealkv` y de `surrealdb`.

## Decisiones pendientes del dueño

- **App móvil:** Tauri 2 móvil con la misma interfaz de Angular y un plugin Kotlin para
  la cámara (CameraX + ML Kit: PDF417 de la cédula, QR y lectura de texto de cédula,
  carnet KOF y placa, como en Lattis), o UniFFI + Kotlin/Compose como Lattis. Decidido
  ya, sea cual sea la opción: **la cámara sólo captura y entrega lo crudo; interpretar,
  validar y decidir es del dominio** (`arquitectura.md`, "La cámara es una herramienta").
- **Sincronización de catálogos** (contratistas, empresas, gafetes, usuarios): no son
  hechos; la propuesta es changefeed por tabla con "el último cambio gana" y la
  auditoría como respaldo.
- **Renombrar una empresa.** El núcleo ya renombra (con auditoría), pero no hay pantalla.
  Opciones: un lápiz junto al «+» del selector que abre el mismo modal (recomendada), una
  lista simple de "Administrar empresas", o corregirlo desde la nube. El dueño dijo que
  alguien debe **auditar** los nombres de las empresas: no se deja solo al guarda.

## Lo que sigue

1. **Decidir cómo se renombran las empresas y quién audita sus nombres** (ver arriba). El
   núcleo ya renombra (`RenombrarEmpresa` y `RenombrarEmpresaProveedora`); falta la
   pantalla y la decisión del dueño.
2. **Mantener las listas al día cuando haya sincronización**: hoy cada pantalla recarga al
   abrirse y tras sus propias acciones; sin botón de actualizar. Con la nube habrá que
   refrescarlas solas.
3. Las exportaciones del Historial (CSV, Excel, PDF), al final y a pedido del dueño.
4. CodeQL para el JavaScript en la CI. Equipos (bloque M) y la nube (con L4 y L5).

## Cómo trabajar en esta máquina

- No hay Visual Studio: compilar con el toolchain GNU. En PowerShell:
  `$env:RUSTUP_TOOLCHAIN='1.99.0-x86_64-pc-windows-gnu'` (no se toca
  `rust-toolchain.toml`, que usa la CI en Linux). El `link` de Git Bash rompe MSVC.
- **Con la app abierta (`tauri dev`) el `target` queda bloqueado:** para `clippy` y
  `cargo test` usar otra carpeta, `$env:CARGO_TARGET_DIR="$env:TEMP\limen-clippy"`.
- Verificar antes de cada commit: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --all -- -D warnings`, `cargo test --all`; y en
  `apps/escritorio`: `npx prettier --check "src/**/*.{ts,html,scss}"`, `npx ng test
  --no-watch` y `npm run build`.
- Las pruebas de Angular corren aisladas (`"isolate": true` en `angular.json`): cada
  archivo simula `@tauri-apps/api/core` a su manera, y sin aislamiento los simulacros de
  un archivo pisaban los de otro (fallaban juntos y pasaban solos). Un `invoke` simulado
  no es una tarea de Angular: para esperar su respuesta usar `vi.waitFor` o un
  `setTimeout(0)` antes de `whenStable`.
- Angular 22 pide Node 22.22.3 o más nuevo (la CI usa Node 24).
- La app en desarrollo: `npx tauri dev` dentro de `apps/escritorio`. Los paquetes npm de
  Tauri deben coincidir en versión menor con el crate de Rust (hoy 2.11).
- Cerrar la app antes de importar datos: la base queda tomada mientras esté abierta.
- Si un cambio de Rust deja el código a medias, `tauri dev` se detiene solo (vigila los
  crates): se relanza al terminar.
- **Escribir archivos con tildes:** en esta PC el shell daña los acentos (y a veces
  inserta caracteres de control) en un `echo`, un heredoc o un `node -e`. Usar la
  herramienta de escribir archivos; con `node -e` sólo patrones sin tildes.

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
