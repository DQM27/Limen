# Panel de administración (`apps/panel`)

Worker de Cloudflare con una página web. Es la parte de administración que en Lattis
vive en su panel: **lo que no se hace en la portería se hace aquí.** La portería nunca
depende de él (`operacion-sin-conexion.md`).

## Orden de construcción

1. ✅ **Dispositivos:** quién está conectado (`conexion`: la app anuncia su conexión y
   avisa que sigue viva cada 5 minutos; `crates/infra-nube/src/presencia.rs`). Estado: en
   línea, caído (más de 10 minutos sin avisar) o fuera.
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
