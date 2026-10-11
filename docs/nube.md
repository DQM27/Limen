# La nube de Limen

Cómo se reparten los datos entre la nube y los equipos, y en qué orden se construye la
sincronización. Complementa la sección 9 de [`arquitectura.md`](arquitectura.md) y
[`operacion-sin-conexion.md`](operacion-sin-conexion.md).

**Estado:** modelo acordado con el dueño. La instancia existe y tiene el esquema; la
sincronización todavía no.

---

## 1. La instancia

| Dato | Valor |
|---|---|
| Servicio | SurrealDB Cloud, organización "Individual", plan Start |
| Instancia | `limen`, tipo `free` (0,25 CPU, 512 MB, 1 GB), región `aws-use1` |
| Versión | SurrealDB 3.3.2, la misma que usan los equipos |
| Namespace y base | `limen` / `principal`, iguales que en los equipos |
| Esquema | el de los equipos (`infra-surreal/src/esquema/esquema.surql`), aplicado tal cual |

La dirección (`wss://…surreal.cloud`) y las credenciales **no van en el repositorio**,
que es público: se leen de variables de entorno (`LIMEN_NUBE_URL`, `LIMEN_NUBE_USUARIO`,
`LIMEN_NUBE_CLAVE`, `LIMEN_NUBE_NIVEL`). Para volver a aplicar el esquema después de
cambiarlo: `cargo run -p limen-infra-nube --bin limen-nube -- esquema` (es declarativo:
aplicarlo otra vez no toca los datos).

## 2. El modelo: la nube manda, el equipo es desechable

- **La nube es la fuente de verdad** de los catálogos, los registros y el historial.
- **El equipo es desechable.** Uno nuevo se vincula, baja lo que le corresponde y queda
  trabajando. Si se pierde o se daña, no se pierde nada que ya haya subido.
- **El equipo guarda una copia local y trabaja sin conexión.** La portería nunca se
  bloquea por la red (`operacion-sin-conexion.md`): sin internet se registra igual, con
  las reglas locales, y lo pendiente se sube al volver la conexión.
- **Tiempo real.** Con conexión, cada equipo escucha los cambios de su unidad operativa
  y su pantalla se actualiza sola.

## 3. Qué es global y qué es de cada unidad operativa

"Unidad operativa" es lo que Lattis llamaba "sitio" (Brisas, Cartago, Belén…). Cada
equipo pertenece a una.

| Dato | Alcance | Por qué |
|---|---|---|
| Contratistas y empresas | **Global**: igual en todas las unidades | Es la misma persona en todos lados: un acceso negado en una unidad queda negado en todas |
| Empresas proveedoras y personal KOF | **Global** | Catálogos, igual que los anteriores |
| Usuarios | **Global**; se administran en la nube | Un operador puede cubrir turno en otra unidad |
| Gafetes | **De la unidad** | Son el inventario físico de cada lugar |
| Ingresos, presencia, préstamos e historial (hechos) | **De la unidad** | Un movimiento pasó en un lugar concreto |
| Equipos | **De la unidad** | Cada equipo trabaja en un solo lugar |

Un equipo baja **los catálogos globales completos** y, de lo que es por unidad, **sólo
lo de la suya**.

## 4. Cómo viajan los datos

1. **Al registrar** (con o sin red): una sola transacción local guarda el estado, el
   hecho y la marca de "pendiente de subir".
2. **Subida:** en segundo plano, lo pendiente se sube en lote. Cada registro tiene un ID
   fijo (UUID v7): un reintento no duplica nada. Lo que la nube acepta queda confirmado.
3. **Bajada en vivo:** con conexión, el equipo escucha (`LIVE SELECT`) los catálogos y
   lo de su unidad. Lo que llega se guarda en la copia local y la pantalla se refresca.
4. **Al volver la conexión:** el equipo pide lo que pasó desde su último cursor
   (`SHOW CHANGES … SINCE`) y lo aplica.
5. **Equipo nuevo:** baja todo lo que le corresponde (sección 3) de una vez y después
   sigue con el paso 3.
6. **Choques entre equipos** (la misma persona adentro dos veces, el mismo gafete
   prestado dos veces): no se borra ni se rechaza nada; el registro queda con el
   indicador de incidente (`operacion-sin-conexion.md`, 4.3 y 4.4).

## 5. Identidad de los equipos

Mientras no exista el bloque M (vinculación con código de un solo uso y par de claves
propio, como en Lattis), **cada equipo entra con su propio usuario de base de datos**,
limitado a lo que le corresponde. Cuando exista el bloque M se reemplaza por la firma del
equipo sin cambiar el flujo de datos.

## 6. Orden de construcción

1. ✅ Conectar y aplicar el esquema (`crates/infra-nube`, herramienta `limen-nube`).
2. **Unidades operativas y equipos** en el dominio y en el esquema: tabla de unidades,
   la unidad en gafetes, ingresos, presencia, préstamos y hechos, y el equipo que
   registró cada hecho.
3. Subida de lo pendiente (bandeja de salida).
4. Bajada: carga inicial de un equipo nuevo, en vivo y recuperación por cursor.
5. Indicador de incidente al unir lo de varios equipos.
6. Usuarios desde la nube (L4, L5) y la hora de la nube para los equipos sin NTP.
