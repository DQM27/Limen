# Para continuar donde se quedó

Lee primero [`AGENTS.md`](../AGENTS.md) (reglas del dueño y de arquitectura) y, si hace
falta el detalle, [`arquitectura.md`](arquitectura.md), [`reglas.md`](reglas.md) y
[`operacion-sin-conexion.md`](operacion-sin-conexion.md).

## Dónde quedó todo

- **Núcleo** terminado: dominio, aplicación, persistencia (memoria y SurrealDB),
  composición, buscador y consultas de lectura. Además, el historial de ingresos por
  rango de fechas y los accesos rápidos de fecha (`dominio/rango_fechas.rs`).
- **Hechos inmutables** (regla E11, sección 9.1 de `arquitectura.md`): cada entrada y
  salida, por las cuatro vías, queda en la tabla `hecho` en la misma transacción que el
  estado. La base no deja editar ni borrar `hecho` ni `auditoria` (`READONLY` y un
  `DEFINE EVENT`). Es la base de la sincronización.
- **App de escritorio** (Tauri 2 + Angular 22 + Angular Material + AG Grid Community):
  - `apps/escritorio/comandos`: la lógica de los comandos (JSON, un solo error para la
    interfaz, la sesión del equipo). Se prueba con la base en memoria.
  - `apps/escritorio/src-tauri`: cascarón de Tauri, sin lógica ni pruebas propias (un
    ejecutable de pruebas que enlaza Tauri no arranca en Windows).
  - Frontend: barra lateral, `compartido/tabla` (la grilla base de toda la app, con el
    diseño unificado) y la pantalla de Contratistas.
- **Usuarios e inicio de sesión** (bloque L, reglas L1–L3 y L6–L8): cédula y
  contraseña, Argon2id (`infra-plataforma`), bloqueo tras 5 fallos, primer usuario del
  equipo y contraseña temporal. El operador provisional (`operador.json`) ya no existe:
  **la app arranca sin sesión** y ningún comando hace nada hasta entrar.
- **Reloj confiable** (regla E5 nueva): la hora sale de NTP (`time.windows.com`, luego
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

## Ramas y PR (nada va a `main` sin PR y CI en verde)

Todo lo anterior está en `main` (PR #3 a #11). `feat/importador-lattis` quedó atrás:
necesita traer `main` y generar los hechos (con la hora sellada) de los ingresos
importados antes de su PR. Las demás ramas viejas ya están fusionadas y se pueden borrar.

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

## Lo que sigue

0. **La pantalla de inicio de sesión va primero**: sin ella la app no puede operar.
   Todo está listo en `nucleo/sesion.ts` (`SesionServicio`: `usuario`, `cargar`,
   `iniciar`, `crearPrimerUsuario`, `cambiarContrasena`, `cerrar`) y en
   `nucleo/comandos.ts` (`hayUsuarios` y la administración de usuarios). Flujo:
   - al abrir, `hayUsuarios()`: sin usuarios, el alta del primero (cédula, nombre y
     contraseña); con usuarios, cédula y contraseña;
   - si `usuario().debe_cambiar_contrasena`, sólo el cambio de contraseña (el núcleo
     rechaza todo lo demás con `contrasena_temporal`);
   - los errores de entrada no traen campo (`credenciales_invalidas`,
     `inicio_bloqueado`, `usuario_desactivado`): mensaje general, tal cual;
   - cualquier comando puede responder `sin_sesion`: volver a la pantalla de entrada.
   - Pantalla de Usuarios (listar, registrar con contraseña temporal, editar nombre y
     activo, restablecer contraseña); los errores traen `campo` (`cedula`, `nombre`,
     `contrasena`, `contrasena_actual`, `activo`).
1. **Historial de ingresos: el núcleo ya está; falta la pantalla.**
   `nucleo/comandos.ts`: `listarHistorial(desde, hasta)` (fechas `AAAA-MM-DD` o `null`;
   trae `movimientos`, `truncado` y `maximo`) y `atajosDeFecha()` (código, etiqueta,
   nombre corto, rango y cuál abre el historial: los últimos 6 meses). Un rango
   invertido rechaza con `campo: 'hasta'`. La pantalla: la grilla base con un selector
   de rango (botón con la etiqueta corta, panel con los atajos y fechas Desde/Hasta, el
   modelo de Lattis). Las exportaciones (CSV, Excel, PDF) quedan para el final, a pedido
   del dueño.
2. **Formulario de "Nuevo contratista"** (diálogo de Material) y el botón en la fila de
   herramientas de la grilla. **El núcleo ya está listo**; falta sólo la pantalla:
   - `nucleo/comandos.ts`: `registrarContratista`, `editarContratista` (devuelve los
     `Cambio` o `[]` si nada cambió), `buscarEmpresas` y `registrarEmpresa`; tipos
     `ContratistaEntrada`, `Empresa` y `Cambio` en `tipos.ts`.
   - Cada error trae `campo` (`cedula`, `nombre`, `empresa_id`, `tipo_ingreso`,
     `fecha_vencimiento_praind`, o `nombre` en la empresa): el mensaje va junto a ese
     campo, sin que la interfaz traduzca códigos. `campo: null` → mensaje general.
   - Para editar, la fila de la grilla ya trae todo para precargar el formulario.
   - **Ingreso de contratista (lo más crítico), listo sin pantalla:** `buscarParaIngreso`
     (cédula o nombre en un solo campo) y `prepararIngreso` traen a cada contratista con
     la decisión tomada por el núcleo (`puede_entrar`, `acceso` con su aviso, o `motivo`
     y `adentro_por` para ofrecer su salida) y sus `gafetes_perdidos`;
     `registrarEntradaContratista` lleva `gafete` o `sin_gafete` (S/G). Si
     `requiere_gafete`, el núcleo exige uno de los dos (`gafete_requerido`); la pantalla
     no decide nada. `dentro()` trae `sin_gafete` para mostrar «S/G».
   - **Lo mismo para las demás pantallas** (comandos, errores por campo y funciones en
     `nucleo/comandos.ts`, todo probado): empresas (`renombrarEmpresa`), empresas
     proveedoras (buscar, registrar, renombrar), entrada de proveedor y por correo,
     personal KOF (buscar, registrar, editar, entregar el provisional) y gafetes
     (listar por tipo, crear por rango, cambiar estado: perdido con su portador, pagado,
     apareció, de baja). Las salidas por las cuatro vías ya existían.
3. ✅ **CI para Node y Tauri**: el trabajo `escritorio` (Windows) instala, revisa el
   formato, prueba y compila Angular, y pasa clippy al cascarón; los demás trabajos
   excluyen `limen-escritorio`. Falta activar CodeQL para el JavaScript.
4. Otras pantallas de la barra lateral (Dentro, Proveedores, Correo, Personal KOF,
   Gafetes), los equipos (bloque M) y la nube (con L4 y L5), en ese orden.

## Cómo trabajar en esta máquina

- No hay Visual Studio: compilar con el toolchain GNU. En PowerShell:
  `$env:RUSTUP_TOOLCHAIN='1.99.0-x86_64-pc-windows-gnu'` (no se toca
  `rust-toolchain.toml`, que usa la CI en Linux). El `link` de Git Bash rompe MSVC.
- Verificar antes de cada commit: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --all -- -D warnings`, `cargo test --all`; y en
  `apps/escritorio`: `npx prettier --check "src/**/*.{ts,html,scss}"`, `npx ng test
  --no-watch` y `npm run build`.
- Las pruebas de Angular corren aisladas (`"isolate": true` en `angular.json`): cada
  archivo simula `@tauri-apps/api/core` a su manera, y sin aislamiento los simulacros de
  un archivo pisaban los de otro (fallaban juntos y pasaban solos).
- Angular 22 pide Node 22.22.3 o más nuevo (la CI usa Node 24).
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
