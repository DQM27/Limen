# Panel de administración (`apps/panel`)

Worker de Cloudflare con una página web. Es la parte de administración que en Lattis
vive en su panel: **lo que no se hace en la portería se hace aquí.** La portería nunca
depende de él (`operacion-sin-conexion.md`).

## Orden de construcción

1. ✅ **Dispositivos:** registrar un equipo (nombre, unidad de una lista y tipo) y emitir su código
   de vinculación (15 minutos, un solo uso), retirarlo, y ver quién está conectado (`conexion`: la
   app anuncia su conexión y avisa cada 5 minutos que sigue viva; en línea, caído si lleva más de 10
   minutos sin avisar, o fuera). Ver también «Acceso del equipo a la nube» más abajo.
2. **Historiales** (sólo lectura): entradas y salidas de la tabla `hecho`, con filtros
   por unidad y fecha.
3. **Usuarios** (bloque L4 y L5): crear, desactivar, clave temporal y desbloqueo. Los
   usuarios son globales y los administra la nube; el equipo sólo inicia sesión.
4. **Contratistas:** administrar el catálogo global (alta, edición, vencimiento de la
   PRAIND, empresa).

## Reglas para construirlo

- **Las reglas viven en `crates/dominio`**, no en JavaScript. Qué cédula es válida, cuándo
  se bloquea un usuario o cuándo una clave es temporal se decide en Rust. El dominio ya
  compila a WebAssembly: el Worker lo carga y sólo orquesta, igual que la app de
  escritorio con sus comandos.
- **Sólo lectura al principio.** Escribir (pasos 3 y 4) pide un usuario de base de datos
  propio con permiso sobre esas tablas y nada más; nunca el administrador.
- **Entrada real:** la clave simple de hoy sirve para ver dispositivos. Para administrar
  usuarios hace falta inicio de sesión con nombre y rol (o Cloudflare Access) y un
  registro de quién cambió qué (`auditoria`).
- Un cambio hecho en el panel llega a los equipos por la sincronización de catálogos
  (`nube.md`); el panel no habla con los equipos.

## Despliegue

Ver `apps/panel/README.md`. Hoy en `workers.dev`; el subdominio propio de megabrisas se
conecta cuando se decida el nombre.

## Acceso del equipo a la nube (token)

Tras vincularse, el equipo **no lleva ninguna clave de la base**. En cada conexión:

1. pide al Worker un desafío (`POST /api/token { equipo }`);
2. lo devuelve firmado con su clave privada P-256 (la que generó al vincularse;
   `clave-equipo.json`);
3. el Worker comprueba la firma contra la clave pública guardada en `dispositivo`, y el dominio
   decide si puede conectarse (vinculado y no retirado, regla M2). Si sí, responde con un token
   de la base y la dirección de la nube; si no, `dispositivo_invalido` (sin decir cuál falló).

La clave de la base vive sólo en los secretos del Worker. Con las variables `LIMEN_NUBE_*` la app
sigue conectándose como antes (desarrollo y herramientas).

**Límite conocido:** retirar un equipo impide que obtenga un token nuevo, pero la conexión que ya
tiene abierta sigue hasta que se cierre. Cortarla al instante (como Lattis) queda pendiente.

## La página: cómo está ordenada

Angular (`apps/panel/web`), con la misma forma que la app de escritorio: **menú lateral con las
secciones** (Equipos, Usuarios, Historiales, Contratistas) y, al lado, la pantalla de la sección.
Las que todavía no existen se ven apagadas en el menú (`navegacion.ts`); cada pantalla nueva sólo
agrega su ruta. Todas siguen el mismo orden: título, una fila de herramientas (acciones propias a
la izquierda, conteo a la derecha) y la tabla AG Grid (`compartido/tabla.ts`). Material 3 con la
paleta Tokyo Night, clara (Day) u oscura (Night) según el sistema, igual que el escritorio.

La página no decide nada: carga el núcleo (el dominio de Rust compilado a WebAssembly,
`web/src/dominio-wasm`, hecho por `npm --prefix apps/panel run dominio`) y le pregunta. El nombre
se ve en mayúscula al escribir, la cédula se normaliza, los mensajes de error son los del núcleo y
el buscador y el resaltado de las tablas son sus reglas N1 y N2. Quedan apagados los filtros por
columna de AG Grid, que compararían por su cuenta: vuelven cuando el núcleo decida también esos
(`celdas_que_cumplen` ya está expuesta). La actualización en vivo falta: hoy se refresca cada 15
segundos.

## Usuarios (bloque L)

`GET` y `POST /api/usuarios`, `PATCH /api/usuarios/:id` (cédula, nombre y activo; desactivar es
`activo = false`: sólo impide iniciar sesión, se revierte con el interruptor y a nadie se borra) y `POST /api/usuarios/:id/clave` (restablecer). El Worker sólo
orquesta (`usuarios.js`): **el núcleo decide** qué cédula y qué nombre valen y cómo quedan
normalizados, que la cédula sea única (la cédula es editable; el usuario conserva su ID), cómo es
la clave temporal (10 símbolos sin 0, O, 1 ni I, del azar que entrega el Worker) y qué cambió para
la auditoría. La clave se cifra en la propia base con `crypto::argon2::generate` (una prueba de
Rust comprueba que el login de Limen acepta ese hash) y se muestra una sola vez; nunca va a la
auditoría. Hasta que entre Cloudflare Access, la auditoría anota los cambios del panel con el
identificador nulo en vez de un usuario con nombre.

## Contratistas (bloque B)

`GET /api/contratistas` (contratistas y empresas), `POST /api/contratistas`, `PATCH
/api/contratistas/:id` y `POST /api/empresas`. El catálogo es global: el panel lo edita en la nube y
los equipos lo bajan solos. **El núcleo decide** (`contratista_evaluar` y `empresa_evaluar` en
`crates/dominio-wasm`, que usan las mismas reglas que el escritorio): qué cédula y nombre valen y
cómo quedan, que la empresa exista, el PRAIND vigente (se mide con el día de Costa Rica; al editar
sólo se revisa si cambió la fecha, así a alguien con el PRAIND vencido se le puede quitar el
acceso), que la cédula sea única, que no se cambie la cédula de quien está adentro, el mensaje y el
campo de cada problema y qué cambió para la auditoría. Si el formulario no vale, el Worker responde
`422` con **todos** los problemas a la vez (`{ problemas: [{ codigo, campo, mensaje }] }`) y la
página marca cada campo. El Worker solo lee lo que la regla necesita saber (¿otro tiene la cédula?,
¿la empresa existe?, ¿está adentro? —mira `presencia`—) y guarda con la auditoría en una
transacción. Un contratista **nunca se borra**: se le quita el acceso (interruptor «Acceso») y se le
puede devolver. En la página, la columna «Vence PRAIND» se pinta según lo que dice el núcleo, la
empresa se elige entre las que existen (filtradas con la búsqueda N2 del núcleo) y una nueva se crea
con «+».

Cosas que costaron un tropiezo: SurrealDB lee como UUID todo texto que lo parezca, así que al
guardar la auditoría (la empresa del alta es un UUID) se devuelve a texto con `<string>`; y la
auditoría es inmutable, por eso las pruebas contra la nube dejan sus entradas de auditoría.
