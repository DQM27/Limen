# Importador de Lattis

Herramienta de desarrollo (`herramientas/importador-lattis`) que carga en Limen un
volcado SQL de Lattis, adaptado a las reglas de Limen. Sirve para tener una base de
pruebas con datos realistas y es el ensayo de la migración real (paso 9 de la hoja de
ruta). Si algún día hay que traer datos de otra app, es la base para hacerlo.

## Los datos reales no entran al repositorio

El repositorio es **público**. Los datos del cliente viven en `datos-privados/`, que git
ignora (igual que `*.sql`, `*.csv` y `*.dump`; Limen usa `.surql` para su esquema). El
programa sólo imprime conteos y códigos de rechazo con el identificador de Lattis del
registro, nunca nombres ni cédulas, y las pruebas usan datos inventados. Tampoco se
documentan aquí cifras ni nombres sacados de los datos reales.

## Uso

1. Volcar las tablas de Lattis a `datos-privados/lattis-produccion/` como
   `NN_tabla.sql` (`INSERT INTO tabla (columnas) VALUES (...), (...);`):

   | Archivo | Columnas |
   |---|---|
   | `01_empresas.sql` | `id, nombre` |
   | `02_contratistas.sql` | `id, identificacion, nombre, empresa_id, tipo_ingreso, fecha_vencimiento_praind, activo, es_personal_ruta` |
   | `03_gafetes.sql` | `id, numero, tipo, estado` |
   | `04_empresas_proveedor.sql` | `id, nombre` |
   | `05_personal_kof.sql` | `id, codigo_empleado, nombre` (en Lattis, `encargados_ruta`) |
   | `06_ingresos_contratista.sql` | `id, contratista_id, cedula, nombre, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero, placa, hora_entrada, hora_salida, resultado_acceso, motivo_resultado, usuario_entrada, usuario_salida` |
   | `07_ingresos_proveedor.sql` | `id, cedula, nombre, empresa_id, empresa_nombre, placa, gafete_numero, hora_entrada, hora_salida, usuario_entrada, usuario_salida` |
   | `08_ingresos_correo.sql` | `id, cedula, nombre, motivo, placa, gafete_numero, hora_entrada, hora_salida, usuario_entrada, usuario_salida` |
   | `09_prestamos_gafete_kof.sql` | `id, personal_id, codigo_empleado, gafete_numero, hora_entrega, hora_devolucion, usuario_entrega, usuario_devolucion` |

   Las fechas van como `2026-09-29 19:48:47+00` y los booleanos como `true`/`false`
   (al generar el SQL con `format('%L')` hay que convertirlos con `::text`: sin eso
   Postgres escribe `t`). Un resultado grande de la consulta se guarda en un archivo del
   disco: conviene procesarlo con un script que verifique el `md5` que calcula el
   servidor, en vez de copiarlo a mano.
2. **Cerrar la app de escritorio** (la base queda tomada mientras esté abierta).
3. `cargo run -p limen-importador-lattis -- --datos datos-privados/lattis-produccion --base <carpeta>`
   (sin `--base`, carga en la base de la app de escritorio de este equipo:
   `%APPDATA%\com.dqm27.limen.escritorio\base`).

El importador no mezcla con lo que ya hay: si la base tenía datos, el choque de
unicidad se informa y hay que borrar la carpeta de la base antes de volver a importar.

## Cómo adapta los datos

Todo pasa por los tipos del dominio, así que lo que rompe una regla se **rechaza con su
código**. Se carga con los constructores `restaurar`, que reconstruyen lo guardado sin
volver a exigir lo que pudo vencer con el tiempo: un contratista con el PRAIND vencido
o sin acceso existe en la realidad y hace falta para probar.

| Lattis | Limen |
|---|---|
| `empresas.activa`, `empresas_proveedor.activa` | No se lee: las empresas no desaparecen (C) |
| `contratistas.es_personal_ruta` | No se lee: el personal de ruta no existe, son contratistas normales |
| `contratistas.activo` | `tiene_acceso` |
| Tipo `SWAT` o `POR_CORREO` (personal de otras empresas, sin PRAIND) | PRAIND con fecha ficticia lejana (2099-12-31) |
| PRAIND sin fecha en un PRAIND o IN HOUSE | Rechazo `praind_sin_fecha` (B5) |
| Medio `CAMINANDO` | A pie |
| Vehículo sin placa, o con una placa que Limen no acepta | Placa inventada `IMP-001` (E2) |
| Ingreso sin gafete | Se conserva (E3); a un IN HOUSE se le ignora el gafete |
| `encargados_ruta` | Personal KOF (código de empleado y nombre) |

Entre filas, el importador cuida lo que el dominio decide con "hechos": cédula, nombre o
código repetidos, empresa que no existe, una persona adentro por dos vías a la vez y un
gafete prestado dos veces. Cada alta queda auditada (B11) a nombre de un operador
"importador". Los préstamos KOF se cargan primero los cerrados (en orden cronológico) y
después los abiertos: la devolución de un préstamo quita la marca de "tiene un préstamo
abierto" de esa persona.
