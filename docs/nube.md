# La nube de Limen

Cómo se reparten los datos entre la nube y los equipos, y en qué orden se construye la
sincronización. Complementa la sección 9 de [`arquitectura.md`](arquitectura.md) y
[`operacion-sin-conexion.md`](operacion-sin-conexion.md).

**Estado:** modelo acordado con el dueño. La instancia existe, tiene el esquema y la
sincronización (subida, bajada y tiempo real) ya funciona, con reintentos de espera
aleatoria (sección 11); falta lo que marca la sección 6 (incidentes y usuarios desde la
nube).

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

Para **subir la base de un equipo** (con la app cerrada, porque la base queda tomada
mientras está abierta):

```text
cargo run -p limen-infra-nube --bin limen-nube -- subir "%APPDATA%\com.dqm27.limen.escritorio"
```

Abre la base (y la convierte si es de antes de las unidades operativas), deja el esquema
de la nube al día y copia, con los mismos IDs, los catálogos, lo de la unidad, los
hechos y la auditoría. Se puede repetir: no duplica nada, y lo que en el equipo ya
terminó (una presencia, un préstamo) deja de estar en la nube. No sube los usuarios (los
de un equipo de desarrollo son de prueba; los reales vienen de la nube), los intentos de
inicio de sesión ni el reloj del equipo.

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

**Hecho (bloque M):** el equipo se vincula con un código de un solo uso, genera su par de claves
P-256 y, en cada conexión, firma un desafío; el Worker del panel comprueba la firma y le da un token
de la base. El equipo no lleva ninguna clave de la base (`panel.md`, «Acceso del equipo a la nube»).
Queda pendiente acotar el token a lo que cada equipo necesita (hoy es un usuario de base compartido
con rol `EDITOR`) y cortar al instante las conexiones de un equipo retirado.

## 6. Orden de construcción

1. ✅ Conectar y aplicar el esquema (`crates/infra-nube`, herramienta `limen-nube`).
2. ✅ **Unidades operativas y equipos** en el dominio y en el esquema (sección 7).
3. ✅ Subida continua (sección 8). A mano está `limen-nube subir`, que usa el mismo
   camino.
4. ✅ Bajada: carga inicial de un equipo nuevo, en tiempo real y al volver la conexión
   (secciones 9 y 10).
5. Indicador de incidente al unir lo de varios equipos.
6. ✅ Usuarios desde la nube: bajan al equipo y el login es local. ⏳ Crearlos y administrarlos desde
   el panel, la hora de la nube para los equipos sin NTP, y L5 (sesión única).
7. ✅ Reintentos con espera aleatoria para que miles de equipos no vuelvan a la vez
   (sección 11).

## 7. Unidad operativa y equipo, como quedaron

- **Dominio** (`dominio/origen.rs`): `UnidadOperativaId` (un código corto y estable,
  `BRISAS`), `EquipoId` (UUID v7) y `Origen`, que junta los dos. Cada **hecho** lleva su
  origen.
- **Sesión:** dice quién, en qué unidad y desde qué equipo. El inicio de sesión le pone
  el origen del equipo, y los casos de uso se lo pasan a cada hecho.
- **Identidad del equipo:** el archivo `equipo.txt` en la carpeta de datos de la app
  (`unidad=BRISAS`, `equipo=<uuid>`). La primera vez que arranca, el equipo lo crea con
  la unidad por omisión (`BRISAS`, `composicion::UNIDAD_POR_OMISION`) y un ID nuevo.
  Con el bloque M llegará al vincular el equipo.
- **Base del equipo:** el almacén conoce su unidad. Las claves de los gafetes y de sus
  préstamos la llevan (`gafete:⟨BRISAS-CONTRATISTA-25⟩`), y la Unit of Work le pone el
  campo `unidad` a cada fila de las tablas de la unidad (`TABLAS_DE_LA_UNIDAD`) al
  confirmar. La presencia sigue siendo una por persona en todas las unidades (A7); su
  campo `unidad` dice en cuál está.
- **Pantallas:** sin cambios. La base local sólo tiene lo de su unidad.
- **Nube:** el esquema nuevo ya está aplicado y existe `unidad_operativa:BRISAS`.

**Bases de antes de este cambio:** no hay que borrarlas. Al abrirse, la base de un
equipo se convierte sola (`infra-surreal/src/esquema/sin_unidad.surql`, sección 8.4 de
`arquitectura.md`): los gafetes y sus préstamos pasan a la clave con la unidad, y lo
demás recibe el campo `unidad`. Sólo toca lo que no tiene unidad, así que en una base
nueva o ya convertida no hace nada. Los hechos no se editan: los que no traen origen se
leen como registrados en ese equipo, que es donde se registraron.

**Pendiente para cuando haya una segunda unidad:** el historial de cambios de un gafete
(auditoría) lo identifica por `TIPO-NÚMERO`, sin la unidad. Con una sola unidad no
importa; con dos, el historial del 25 de Brisas se mezclaría con el del 25 de la otra.

## 8. Subida continua, como quedó

- **Bandeja de salida (patrón «outbox», como `cola_salida` en Lattis).** Cada tabla que se
  comparte tiene un evento (`DEFINE EVENT salir`, en `crates/infra-surreal/src/esquema/equipo.surql`,
  que sólo se aplica a la base de un equipo) que anota en la tabla local `salida`, **en la misma
  transacción**, que ese registro hay que mandarlo a la nube. Una fila por registro, con su
  último estado y una `version`. Lo que **baja** de la nube se escribe marcado `$de_la_nube` y el
  evento no anota nada: no hay rebote por construcción. Si otro código escribiera en estas tablas
  sin marcarlo, su cambio sube (lo seguro es subir de más, nunca perder un cambio).
- **Al arrancar**, si el equipo tiene la nube configurada (las variables
  `LIMEN_NUBE_URL`, `LIMEN_NUBE_USUARIO`, `LIMEN_NUBE_CLAVE` y `LIMEN_NUBE_NIVEL` del
  usuario de Windows), la app lanza la sincronización en segundo plano
  (`infra_nube::sincronizar`). Sin esa configuración trabaja igual que antes, sólo en el
  equipo.
- **Al instante** (ver sección 10) vacía la bandeja, tabla por tabla (los catálogos antes que lo
  que los usa) y de a 200: lo guardado se crea o se reemplaza en la nube; lo borrado se borra
  (hechos y auditoría nunca se borran: se agregan los que faltan). Un lote se quita de la
  bandeja **después** de confirmarse en la nube, y sólo si la `version` no cambió mientras se
  mandaba: si algo falla se reintenta, y un cambio hecho durante el envío no se pierde.
- **La primera vez**, la bandeja recibe lo que el equipo ya tenía antes de que existiera (una
  base de antes, o lo que se registró sin conexión antes de vincularse): los eventos sólo ven
  lo que se escribe desde que existen. Se hace una sola vez (`sincronizacion:⟨bandeja-sembrada⟩`).
  Una instalación nueva no tiene nada local, así que no sube nada. Todo se puede repetir sin
  duplicar nada.
- **Sin red**, la sincronización sólo lo anota en el registro técnico y reintenta con espera
  creciente. Lo registrado mientras tanto queda en la bandeja y sube al volver la conexión, **sin
  límite de tiempo** (ya no depende de los 30 días del registro de cambios de SurrealDB). La
  portería nunca espera.
- **Sólo crea o reemplaza en la nube**: nunca borra lo que no esté anotado como borrado,
  porque allí también está lo de los demás equipos.

## 9. Bajada, como quedó

- En cada vuelta la app **primero sube y después baja**: si el equipo
  cambió algo sin conexión, la nube recibe su versión antes de que el equipo repase lo
  que cambió allá, y el repaso termina en lo más reciente.
- Baja con el **registro de cambios de la nube**, desde el último que bajó
  (`sincronizacion:⟨bajada-tabla⟩`). Lo mismo sirve en vivo y al volver la red.
- Baja los **catálogos completos** y, de lo que es de una unidad, **sólo lo de la suya**
  (filtra por el campo `unidad`).
- **Equipo nuevo:** la primera vez baja todo lo que le corresponde y arranca al día.
- **Sin rebotes, por construcción:** lo que llega y ya está igual no se escribe, y lo que sí se
  escribe va marcado `$de_la_nube` (`AlmacenSurreal::aplicar_de_la_nube`), así que los eventos de
  la bandeja de salida no lo anotan y nunca vuelve a subir. El 14 de octubre una instalación
  nueva bajó el catálogo (5 307 filas) y enseguida lo volvió a subir, porque la subida leía el
  registro de cambios de SurrealDB, que anota **toda** escritura sin saber de dónde viene: miles
  de escrituras de rebote que además duplicaron el diario de la nube. La solución no es un
  filtro: es el patrón de Lattis, una bandeja que sólo llenan las escrituras que nacen en el
  equipo. Hechos y auditoría sólo se agregan.
- **Pantallas:** cuando lo que bajó cambió algo, el núcleo manda el evento
  `nube-cambios` y Dentro (con Proveedores, Correo y KOF), Contratistas, Gafetes e
  Historial vuelven a cargar.
- **Choques:** si dos equipos cambian lo mismo, queda el último que llegó a la nube. Los
  choques de reglas entre equipos (la misma persona adentro dos veces, el mismo gafete
  prestado dos veces) todavía no se marcan como incidente: es el paso 5.

## 10. Tiempo real, como quedó

La sincronización no espera un reloj: **despierta por lo que pase primero**.

1. **La app guarda algo** en el equipo: la base local avisa al confirmar
   (`AlmacenSurreal::esperar_confirmacion`) y se sube en el momento.
2. **La nube avisa en vivo** (`LIVE SELECT`, `infra_nube::Nube::escuchar`) que otro equipo
   cambió algo: los catálogos completos y, de lo que es de una unidad, sólo lo de la del
   equipo. Se baja en el momento y las pantallas se recargan.
3. **Red de seguridad:** si en 10 segundos no llegó ningún aviso, se sincroniza igual. Los
   avisos en vivo se pierden si se corta la conexión aunque sea un momento; lo que se baja
   sale siempre del registro de cambios, así que la vuelta periódica recupera exactamente
   lo que faltó.

Probado con dos equipos y una nube en memoria: lo que registra uno aparece en el otro en
menos de dos segundos, sin esperar la red de seguridad.

**Choques al escribir a la vez:** si dos equipos escriben al mismo tiempo, la nube puede
pedir "reintente la transacción". La suscripción en vivo y cada vuelta se reintentan
enseguida unas veces; si la suscripción no se pudo crear, se vuelve a intentar en cada
vuelta y, mientras tanto, queda la red de seguridad.

## 11. Escala y reintentos: decisión

Pensando en muchas unidades operativas, y quizá todo el país, se evaluó qué proteger a la
nube de la saturación. **Decisión: por ahora no se agrega ni Kafka ni Nginx.**

| Opción | Por qué no, por ahora |
|---|---|
| **Kafka** | Su valor aquí sería amortiguar picos de escritura, pero ya lo hacen los equipos: cada uno escribe primero en su base local y sube en lotes desde su propia cola (sección 8), con IDs fijos que hacen seguros los reintentos. El `CHANGEFEED` y el `LIVE SELECT` cubren el papel de log ordenado y suscripciones. Un clúster costaría más que toda la nube y sumaría un segundo sistema de verdad que podría desincronizarse. Tampoco resuelve los choques entre equipos: esos son regla de negocio (A7, E1) y se marcan con el indicador de incidente |
| **Nginx** | Balancea servidores de aplicación sin estado. Aquí los equipos hablan directo con SurrealDB; el cuello de botella sería la base (un solo nodo), y un proxy delante de un solo nodo no le quita carga |

**Lo que sí se hace para escalar, en este orden y medido antes de adelantarse:**

1. **Espera aleatoria (*jitter*) en los reintentos.** Tras una caída de la nube, miles de
   equipos que reintentan con el mismo intervalo vuelven al mismo instante y la nube, que
   intenta recuperarse, recibe todo junto (*estampida*). Con espera exponencial y
   aleatoria (*full jitter*: un valor al azar entre 0 y el techo, con tope de unos 5
   minutos) las reconexiones se reparten en el tiempo.
2. **Filtrar los avisos en vivo por unidad** (ya está: `WHERE unidad = …`), para que cada
   equipo reciba sólo lo suyo.
3. **Escala vertical** de la instancia y, si hiciera falta, **partir por región**
   (un namespace o base por zona).
4. **Prueba de carga** con muchos equipos simulados contra la instancia de desarrollo
   antes de elegir el tamaño de producción.

**Cómo quedó** (`crates/infra-nube/src/espera.rs` y `sincronizacion.rs`):

| Dónde | Antes | Ahora |
|---|---|---|
| Reconexión (`sincronizar`) | 30 s fijos | Retroceso exponencial con *full jitter*: el techo empieza en `REINTENTO` (30 s), se duplica con cada fallo seguido y se detiene en 5 minutos; la espera es un valor al azar entre 1 s y ese techo. Al lograr conectar, el contador vuelve a cero |
| `RED_DE_SEGURIDAD` | 10 s fijos | 10 s ±15 %, calculado en cada vuelta, para que equipos arrancados juntos no queden sincronizados entre sí |
| Esquema al reconectar | Se reaplicaba en cada reconexión | Sólo en la primera conexión buena de la sesión (si falla, cuenta como fallo y se reintenta) |
| `una_vuelta_con_reintentos` y `escuchar` | 200 ms × intento, pocas veces | Igual: es para choques de transacción entre pocos equipos y no justifica el cambio |

- **Piso de 1 s:** con *full jitter* el azar puede dar casi cero; el piso evita reintentar
  en un ciclo apretado si eso se repite.
- **Azar inyectable:** el trait `Azar` separa el generador de la lógica. `AzarDelSistema`
  usa `rand`; las pruebas usan uno fijo y comprueban los extremos, el tope y que no haya
  desbordes. Una prueba con azar real verifica que 200 equipos se reparten en muchos
  instantes distintos.
- **Dónde vive:** es infraestructura, va en `infra-nube`, nunca en el dominio (que no
  genera números al azar).
- **Qué falta:** la prueba de carga con muchos equipos simulados (punto 4 de la lista de
  arriba) para elegir el tamaño de producción.

**Dependencia `rand`** (0.9, la misma que ya traía el árbol de forma indirecta, para no
duplicarla). Hoy la usa sólo `espera.rs`. **Ojo con el cifrado:** `rand` sirve para el
jitter, pero claves, sales y nonces deben salir del generador del sistema operativo
(`getrandom`/`OsRng`) o de los crates de cifrado, nunca de un generador previsible; que
`rand` esté a mano no es motivo para usarla ahí.

## 12. Lo que aprendimos de SurrealDB (sincronización)

- **`SHOW CHANGES FOR TABLE … LIMIT n` aplica el límite *antes* de filtrar por tabla.** Si entre el
  cursor de una tabla tranquila (`hecho`) y su próxima entrada hay más de `n` transacciones de otras
  tablas, devuelve cero y el cursor no avanza nunca (el hecho de una entrada se quedó sin subir).
  Por eso la subida y la bajada usan **un solo cursor para toda la base**
  (`SHOW CHANGES FOR DATABASE`) y reparten los cambios por tabla.
- **El registro guarda cada versión de una fila.** Aplicarlas todas escribía versiones intermedias
  que rebotaban entre el equipo y la nube sin parar. Por tanda sólo se aplica la última versión de
  cada fila (`bajada::colapsar`).
- Un texto con forma de UUID que llega como parámetro JSON se convierte en `uuid`: los campos de
  texto necesitan `<string>$id`.
- Un usuario de base de datos no entra con Basic auth: hay que pasar por `/signin` y usar el token.
- Con `DEFINE USER … ROLES VIEWER` se lee todo (incluida `usuario`), aunque las tablas tengan
  `PERMISSIONS NONE`.

## 13. Aislar las unidades operativas desde la base (hecho)

Cada equipo ve y escribe **sólo lo de su unidad** y lo global; lo decide la base, no la app. Antes
el equipo recibía el historial de todas las unidades y la app descartaba lo ajeno.

**Cómo funciona**

1. El Worker del panel firma un token (JWT HS512, una hora) con la **unidad** del equipo y su ID,
   con el secreto `TOKEN_SECRETO` (sólo lo conocen el Worker y la base). Un equipo retirado no puede
   renovarlo: pierde el acceso a más tardar en una hora.
2. La base lo acepta con `DEFINE ACCESS equipo ... TYPE RECORD WITH JWT ... AUTHENTICATE { RETURN
   type::record('equipo', $token.equipo) }`. Con `TYPE JWT` a secas la sesión **no** se trata como
   usuario con permisos por fila y ve todo: hace falta `RECORD WITH JWT` y `AUTHENTICATE`.
   Esa definición lleva la clave de firma, así que **no está en el repositorio**: se define aparte
   (SurrealQL de abajo).
3. `crates/infra-nube/src/esquema_nube.surql` (se aplica después de `esquema.surql`, siempre juntos
   con `limen-nube esquema`) pone los permisos: lo de la unidad `WHERE unidad = $token.unidad`; los
   catálogos globales se leen, se agregan y se editan pero no se borran; los hechos y la auditoría
   sólo se agregan; los usuarios sólo se leen.
4. **El diario de cambios** es una tabla propia, `cambio` (unidad, tabla, registro, tipo, hora), que
   la base llena sola con un `DEFINE EVENT anotar` en cada tabla compartida, en la misma transacción.
   El equipo la lee con sus permisos (su unidad y lo global). `SHOW CHANGES` de SurrealDB **está
   prohibido** para un equipo con acceso limitado, y además guarda una copia de cada fila; el diario
   sólo guarda qué fila cambió. La bajada lee el diario desde su cursor (con un margen de 60 s por si
   dos transacciones se anotan en otro orden), toma de cada fila la última anotación y baja la fila
   tal como está. El aviso en tiempo real es un solo `LIVE SELECT` sobre el diario.
5. **La subida** vacía la bandeja de salida del equipo (sección 8) y escribe en la nube con sus
   permisos. (Hasta el 14 de octubre leía el registro de cambios local de SurrealDB; por eso
   rebotaba lo que bajaba.)

**Lo que se comprobó** (pruebas en `crates/infra-nube`, con una base en memoria, y contra la nube real):

- Con el token de su unidad el equipo sólo ve y escribe lo suyo (los tokens de otra unidad ven 0
  hechos y 0 ingresos, y crear algo a nombre de otra unidad se rechaza).
- **Regla A7:** una persona adentro en otra unidad no se puede registrar en la tuya, aunque el
  equipo no la vea: la clave natural de la presencia no lleva la unidad. La nube ignora ese registro
  sin romper la subida de lo demás. **Pendiente:** anotarlo como incidente (paso 5 de la sección 6).
- Un token alterado no entra. Un equipo con token sube lo suyo, baja lo global y lo de su unidad, y
  de otra unidad no recibe nada. El aviso en vivo le llega con su token y sólo avisa lo suyo.

**Para activarlo en una nube** (una sola vez, como administrador; la clave sale de `TOKEN_SECRETO`):

```sql
DEFINE ACCESS OVERWRITE equipo ON DATABASE TYPE RECORD WITH JWT ALGORITHM HS512
    KEY '<TOKEN_SECRETO>'
    AUTHENTICATE { RETURN type::record('equipo', $token.equipo) }
    DURATION FOR SESSION 1h;
```

y `limen-nube esquema` con un usuario `EDITOR` o superior. Después, en el Worker:
`wrangler secret put TOKEN_SECRETO` y `npm run publicar`.

**Lo que falta**

- Los cambios que haya en la nube **antes** de aplicar los eventos no están en el diario: un equipo
  que ya estaba instalado y no hizo su carga inicial de esa tabla se la perdería. Los equipos
  nuevos hacen su carga completa, así que no les afecta.
- El diario no se purga: crece con cada cambio (anotaciones pequeñas). Decidir una retención y
  purgar lo que todos los equipos ya leyeron.
- Un equipo sin conexión por más tiempo que el diario retenga debería volver a hacer su carga
  completa (cuando exista la purga).
- Las tablas de los equipos (`dispositivo`, `conexion`, …) las lee sólo el panel.

## 14. El panel en tiempo real (hecho)

La página del panel se conecta **directo a la base** (WebSocket, `wss://…/rpc`) y la base le avisa al
instante de lo que cambia (`LIVE SELECT` de `conexion`, `usuario`, `dispositivo` y `hecho`). Con eso
la página actualiza sus tablas sin esperar y muestra avisos (usuario desactivado, equipo que se
conecta, se desconecta o pierde la señal, ingreso nuevo…), con los mismos toasts del escritorio.

**Cómo funciona**

1. La página pide al Worker `POST /api/en-vivo` (con la clave del panel) y recibe un token de **sólo
   lectura** de una hora, firmado con `PANEL_VIVO_SECRETO` (un secreto aparte del de los equipos). Antes
   de que venza, la página pide otro y se vuelve a conectar; si se corta, reintenta sola.
2. La base lo acepta con `DEFINE ACCESS panel ... TYPE RECORD WITH JWT` (igual que el de los equipos:
   lleva la clave de firma, así que **no está en el repositorio**; ver abajo).
3. **Lo que el panel puede hacer lo decide la base**, no la página (`crates/infra-nube/src/esquema_nube.surql`):
   ve los hechos de todas las unidades, los equipos y su conexión y los usuarios, pero **nunca la clave
   (el hash) de un usuario**, y no escribe nada. Los permisos «para todos» (`FULL`) de los catálogos y de
   la auditoría valían también para el panel; ahora escribir queda en `$access = 'equipo'`.
   `crates/infra-nube/tests/panel_en_vivo.rs` lo comprueba con una base en memoria: el panel lee y no
   escribe, y los equipos siguen viendo sólo su unidad, leyendo la clave de los usuarios y editando los
   catálogos.
4. Cómo está conectado un equipo (en línea, sin señal, desconectado) lo decide el núcleo
   (`dominio/conexion_equipo.rs`): el Worker lo calcula al leer y la página lo recalcula con la hora de
   ahora, porque un equipo que deja de avisar no manda ningún cambio.

**Para activarlo en una nube** (una sola vez; la clave sale de `PANEL_VIVO_SECRETO`):

```sql
DEFINE ACCESS OVERWRITE panel ON DATABASE TYPE RECORD WITH JWT ALGORITHM HS512
    KEY '<PANEL_VIVO_SECRETO>'
    AUTHENTICATE { RETURN type::record('panel_lector', 'panel') }
    DURATION FOR SESSION 1h;
```

como administrador de la base (la consola de SurrealDB Cloud), y `limen-nube esquema` con un usuario
`EDITOR` o superior para los permisos. Después, en el Worker: `wrangler secret put PANEL_VIVO_SECRETO` y
`npm run publicar`. Sin el secreto, `/api/en-vivo` responde 503 y la página funciona igual, sin tiempo real
(se actualiza cada 15 segundos).

**Lo que falta**

- Las alertas sólo ven lo que ocurre mientras la página está abierta: no hay historial de alertas (eso
  será la sección Historiales).
- Un aviso de un ingreso no lleva todavía el nombre de la persona, sólo la vía, la unidad y la hora.

### Tropiezo: la presencia de un equipo vinculado (14 de octubre)

Los permisos del acceso del panel dejaron `conexion` sin escritura para todos: un equipo **vinculado** (con su token) no podía anunciar que estaba conectado y SurrealDB descartaba la escritura **sin dar error**, así que el panel nunca lo veía en línea. No se notó porque hasta entonces el equipo de desarrollo entraba con las credenciales de la administración (`LIMEN_NUBE_*`), que se saltan los permisos. Ahora cada equipo escribe sólo su propia presencia (`conexion:⟨su equipo⟩`) y en su unidad; lo prueba `tests/panel_en_vivo.rs`. Lección: probar los permisos con el token del equipo, no sólo con el del panel.
