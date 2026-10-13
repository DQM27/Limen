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
    Enter). Quién devuelve el gafete lo decide el núcleo (regla F8): si el mismo número
    está en uso en dos vías no se adivina y pide usar el botón de la fila. Con un modal abierto los atajos de atrás no valen.
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

## La nube (rama `feat/nube`)

Modelo acordado con el dueño y estado en [`nube.md`](nube.md): la nube es la fuente de
verdad, el equipo es desechable pero guarda su copia local y trabaja sin conexión, los
catálogos son **globales** y lo demás es **de cada unidad operativa** (el "sitio" de
Lattis). La instancia `limen` (SurrealDB Cloud, gratis, `aws-use1`) ya existe con el
esquema aplicado; `crates/infra-nube` conecta y aplica el esquema. Por ahora hay una
sola unidad, **Brisas**. Cada hecho ya lleva su unidad y su equipo, y lo de la unidad lleva
su campo `unidad` (la identidad del equipo está en `equipo.txt`, en la carpeta de datos).
La app ya **sincroniza sola y en tiempo real** con la nube: sube lo que se registra en
el momento y la nube le avisa en vivo lo de los demás equipos (catálogos y lo de su
unidad), con una vuelta cada 10 segundos (±15 %) como red de seguridad; sin red se
acumula y, al reconectar, espera un tiempo al azar que crece con los fallos (para no
saturar a la nube tras una caída; `nube.md`, sección 11). Las pantallas se recargan cuando
llega algo. Falta la prueba de carga con muchos equipos. Lo siguiente es el paso 5 de `nube.md`: marcar
como incidente los choques entre equipos.

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
- **Renombrar una empresa.** El núcleo ya renombra (con auditoría), pero no hay pantalla.
  Opciones: un lápiz junto al «+» del selector que abre el mismo modal (recomendada), una
  lista simple de "Administrar empresas", o corregirlo desde la nube. El dueño dijo que
  alguien debe **auditar** los nombres de las empresas: no se deja solo al guarda.

## Lo que sigue

1. **Decidir cómo se renombran las empresas y quién audita sus nombres** (ver arriba). El
   núcleo ya renombra (`RenombrarEmpresa` y `RenombrarEmpresaProveedora`); falta la
   pantalla y la decisión del dueño.
2. ✅ Las listas se mantienen al día solas: cuando baja algo de la nube, el núcleo manda
   `nube-cambios` y las pantallas recargan.
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

## Sesión del 11 y 12 de octubre de 2026 (lo último)

Ninguna de estas ramas tiene PR todavía; el dueño dirá cuándo.

| Rama | Qué tiene |
|---|---|
| `feat/nube` | Sincronización en tiempo real; reintentos con espera aleatoria (*jitter*, `infra-nube/src/espera.rs`); decisión de escala sin Kafka ni Nginx (`nube.md`, sección 11) |
| `feat/resaltado` (sale de `main`) | **N1:** resaltado de lo que coincide con el buscador. **N2:** el buscador y los filtros de columna de la tabla los decide el núcleo; `pliegue.ts` se eliminó. Comandos `resaltar_coincidencias`, `filas_que_coinciden` y `celdas_que_cumplen` |

### Lo que falta verificar de `feat/resaltado` (en la nube no se pudo)

- Compilar `apps/escritorio/src-tauri`: el contenedor de la sesión no tenía las librerías
  GTK. Los cambios ahí son tres funciones cortas con el patrón de los demás comandos.
- **Ver la app corriendo:** el aspecto de las marcas (`<mark>`) en modo claro y oscuro, y la
  sensación de la pausa de 150 ms al teclear con la tabla llena.
- **Rendimiento con muchas filas:** cada pregunta del buscador envía los textos de todas
  las filas al núcleo. Medir con 100 000 a 500 000 contratistas sintéticos (va con la prueba
  de rendimiento del buscador, abajo).

### Hallazgo: el motor SurrealKV tiene un problema de durabilidad conocido

Verificado en las fuentes originales (no sólo en resúmenes):

- La documentación oficial marca SurrealKV como **beta** y recomienda RocksDB para producción
  prudente en servidor.
- [Issue #428 de `surrealkv`](https://github.com/surrealdb/surrealkv/issues/428): en la
  **0.21.4 (la que usamos)**, un commit confirmado puede **perderse tras un cierre brusco**
  (caída, `SIGKILL`, o el proceso termina antes de cerrar la base), incluso con la
  durabilidad máxima, si ese commit coincide con la rotación de la memoria de escritura.
  Tras un cierre limpio no se pierde nada. Corregido sólo en `main` de `surrealkv`; la última
  versión publicada sigue siendo la 0.21.4. El reporte no probó cortes de energía.
- [PR #420](https://github.com/surrealdb/surrealkv/pull/420): otros errores de recuperación
  del registro de escrituras, también sólo en `main`.
- **Por qué nos toca:** `AlmacenSurreal` no puede esperar un cierre (ver `arquitectura.md`,
  8.2), así que al cerrar la app el proceso puede terminar antes. Se pierde **una sola
  escritura, en silencio**. Con nuestro volumen bajo es poco frecuente, pero no está medido.
- **No se cambia el motor por defecto todavía.** RocksDB perdería el camino del cifrado en
  reposo (TDE de SurrealKV; el cifrado de RocksDB no está conectado en SurrealDB) y complica
  Android. Mientras tanto: medir, cerrar ordenado si se puede, y actualizar cuando salga la
  versión con las correcciones (junto con el cifrado, que ya estaba en espera). Ya existe
  `surrealdb 3.3.3` (usamos la 3.3.2): revisar sus notas antes de subir.

### Experimento en curso: compilar y probar RocksDB en esta PC

Es una **prueba**, en la rama `exp/rocksdb` (sale de `main`); no se fusiona ni cambia el motor
por defecto. Esta PC compila con el toolchain GNU y sin Visual Studio (ver "Cómo trabajar en
esta máquina"): RocksDB necesita un compilador de C++ y `libclang`, así que primero hay que
ver qué hay instalado y avisar al dueño de lo que falte, sin instalar nada a ciegas.

1. **Entorno:** anotar `rustc -vV`, `g++ --version`, `clang --version` y `cmake --version`
   (y `LIBCLANG_PATH` si hace falta).
2. **Compilar:** activar `kv-rocksdb` en `surrealdb` (`Cargo.toml` del workspace) y agregar
   un constructor experimental con `surrealdb::engine::local::RocksDb` junto a `en_disco`
   en `infra-surreal/src/almacen.rs`. Medir: tiempo de compilación en frío y el incremental
   (`cargo build --timings`), tamaño de `target` y del ejecutable en `--release`, frente a
   SurrealKV. Si falla, guardar el error tal cual.
3. **Pruebas:** que las pruebas de `infra-surreal` pasen con una base en disco en los dos
   motores.
4. **Cierre brusco** (lo que de verdad se quiere saber): un programa de prueba que abre la
   base, confirma hechos uno por uno imprimiendo cuántos confirmó, y un proceso padre que lo
   mata a destiempo (`taskkill /F`) y lo reabre para contar cuántos confirmados faltan.
   Al menos 100 repeticiones por motor y con dos tamaños de escritura: realista (unos 300 B
   por hecho) y grande (transacciones de ~500 filas, que fuerzan la rotación y reproducen el
   reporte #428). Matar el proceso no es lo mismo que cortar la energía: decirlo en el
   informe.
   **Repetir cada combinación variando el parámetro `sync`** de SurrealDB (`never`,
   `every` o una duración como `5s`), que según su documentación decide cuándo se vuelca
   la base al sistema de archivos. Averiguar además **cuál es el valor por omisión con el
   que abre hoy** `AlmacenSurreal::en_disco` (se abre sin parámetros) y cómo se pasa en el
   modo embebido. RocksDB, por su parte, escribe de forma asíncrona por omisión: un cierre
   del proceso no pierde nada, pero una caída de la máquina puede perder las últimas
   escrituras ([wiki de RocksDB](https://github.com/facebook/rocksdb/wiki/Basic-Operations)).
   Esto puede pesar más que la elección del motor.
5. **Informe:** `docs/motor-de-base-de-datos.md` con los números y una recomendación. El
   dueño decide; el motor por defecto no cambia en esta prueba. Probar en Android queda
   aparte.

### Clave-valor: ajustes del equipo (esperando al dueño)

`AGENTS.md` dice que el almacén clave-valor se define cuando la pantalla diga qué necesita.
El dueño **todavía está entendiendo cómo funciona** el clave-valor en SurrealDB, así que
primero explicarlo simple, con ejemplos del propio código, y recién después diseñar.

- **Cómo funciona:** SurrealDB no tiene un clave-valor aparte; el ID del registro es la
  clave. `presencia:⟨cédula⟩` es una consulta directa por clave (`via_adentro`), y crear
  una clave que ya existe falla, y de eso se vale "una persona adentro una sola vez".
  `id_reloj()` es un registro único de ID fijo. Ver `infra-surreal/src/repositorios.rs`.
- **Qué NO puede ir ahí:** lo que se necesita antes de abrir la base (`equipo.txt`,
  `reloj-desfase.txt`).
- **Candidatos:** el diseño de las tablas (columnas, anchos), hoy en `localStorage` del
  navegador, y las preferencias de cada operador (hoy no existen).
- **Propuesta (sin aprobar):** ajustes **tipados**, no una bolsa de texto; un puerto con
  implementación en `infra-memoria` y en `infra-surreal` y su prueba de contrato; la
  validación en el dominio; y decidir desde el nombre de la tabla si es sólo del equipo (como
  `sincronizacion`, que no se comparte) o por operador (que se sincronizaría).
- **No implementar sin que el dueño apruebe el diseño y el primer ajuste concreto.**

### Otros pendientes que salieron

- **Índice por fecha del historial:** no hay índice sobre `entrada_en`; la consulta recorre
  la tabla. Proponer `(unidad, entrada_en)` y comprobarlo con `EXPLAIN`.
- **Filtros combinados del historial** (`FiltroHistorial` en el dominio, paginación por
  cursor) y la consulta "quién estaba adentro el martes a las 3 pm". Primero un borrador en
  `docs/` para que lo revise el dueño.
- **Aviso de reloj en la interfaz** (hora sin comprobar, PC desfasada), la **hora de la nube
  como fuente del reloj** para los equipos sin NTP, y `recibido_en` asignado por la nube.
- **Prueba de carga de la sincronización** con muchos equipos simulados.

### Cómo le gusta trabajar al dueño (aprendido en esta sesión)

- **No se duplican reglas.** Si una regla está en el dominio, la pantalla no la repite
  (por eso se eliminó `pliegue.ts`). Si aparece una duplicada, se corrige.
- Cuando dice que no entiende algo, explicarlo **simple y con ejemplos del propio código**,
  sin mezclar conceptos.
- Las decisiones se **documentan en `docs/`**, con lo verificado separado de lo supuesto.
- Verificar contra la fuente (documentación, código, reportes originales) antes de afirmar.

