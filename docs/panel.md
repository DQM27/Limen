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

## Persona adentro en otra unidad (regla A9)

`GET /api/presencia/:clave` (la clave es la cédula, o `KOF-` y el código de empleado). Va con
el token del equipo como Bearer, no con la clave del panel: el Worker lo verifica
(`verificarJwt`) y mira la presencia de **todas** las unidades, cosa que un equipo no puede
hacer por sí solo (la base sólo le deja ver la suya). Responde `{ adentro: null }` o
`{ adentro: { unidad, via, desde } }`: nada más de la persona. Qué hacer con la respuesta lo
decide el dominio (`presencia::verificar_no_adentro_en_otra_unidad`).
