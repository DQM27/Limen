# Operación sin conexión

Decisión de diseño sobre qué pasa cuando un equipo registra un movimiento y no puede
hablar con la nube ni con los demás equipos. Se anota aquí para que cualquier sesión
futura la respete al construir la sincronización y las pantallas.

**Estado:** acordada y confirmada con el dueño.

---

## 1. El problema, tal como lo vive Lattis

Con la nube configurada y sin conexión, Lattis **no deja registrar** un ingreso o una
entrega de gafete provisional, porque no puede validar contra la nube si el gafete está
prestado en otro equipo o si la persona ya está adentro en otro sitio. El operador queda
parado frente a una persona que está físicamente en la puerta.

En el código de Lattis (`src/application/con_nube.rs`):

- Si la consulta a la nube falla, el resultado es `BloqueoIngreso::SinVerificarEnLaNube`
  y no se registra: "con nube configurada no se registra sin verificar (decisión del
  dueño, igual que el gafete)". Hay una prueba que lo fija
  (`sin_respuesta_de_la_nube_no_se_deja_pasar`).
- Para visitas con gafete pasa lo mismo: `Err(error) if gafete_numero.is_some() => return
  Err(error.into())`.
- Pero `docs/pendientes.md` dice lo contrario del chequeo entre sitios: "Sin red, no
  bloquea — sigue local (así lo pedía el pendiente original)", con un aviso simétrico
  después de sincronizar (`contratistas_con_conflicto_activo`).

Es decir, Lattis se contradice a sí mismo: pretende funcionar sin conexión y a la vez
exige conexión para registrar. Limen no hereda esa contradicción.

## 2. Principio

> **La portería nunca se bloquea por la red.**

Dos razones:

1. **El hecho físico ya ocurrió.** La persona está en la puerta o ya entró. Negarse a
   registrarla no la hace desaparecer: sólo deja un hueco en el registro. En una
   emergencia, la lista "dentro" tiene que incluir a todos; una persona que entró sin
   quedar registrada es la peor falla posible de este sistema.
2. **No se puede "des-entrar" a nadie.** Si dos equipos registran lo mismo sin saberlo,
   no hay forma de revertir el hecho; sólo de **reconocerlo y corregir el registro**.

## 3. Qué dice la práctica

No hay una norma única para esto, pero las fuentes coinciden en lo esencial
(las conclusiones que mezclan varias fuentes son nuestra síntesis):

- **Local primero.** En un diseño offline-first la base local es la fuente de verdad y
  la red es sólo una optimización
  ([Educative](https://www.educative.io/courses/mobile-system-design/offline-first-design-and-data-synchronization)).
- **Detectar el conflicto al sincronizar, no impedir el registro.** "Last wins" por
  omisión pisa datos en silencio; lo recomendado es detectar el conflicto (por ejemplo
  con un número de revisión) y aplicar una resolución explícita
  ([Mendix](https://docs.mendix.com/refguide/mobile/building-efficient-mobile-apps/offlinefirst-data/best-practices/)).
  Y borrar con una marca ("borrado lógico") para poder resolver bien después (misma fuente).
- **La disponibilidad se decide por operación.** Un ejemplo de estacionamiento: la
  reserva en línea evita el sobrecupo, pero "los dispositivos de la barrera deben dejar
  entrar y salir sin importar nada; se guarda en caché y se concilia después"
  ([notas de CAP, Universidad de Washington](https://courses.cs.washington.edu/courses/csep552/13sp/lectures/7/cap_intro.pdf)).
  La portería es la barrera, no la reserva.
- **Operaciones provisionales y disculpas.** Pat Helland y Dave Campbell, en
  *Building on Quicksand*, tratan las inconsistencias y su corrección como práctica
  ordinaria del negocio: se acepta, se confirma o se corrige después
  ([resumen](https://maxgrinev.com/2010/01); artículo de Helland:
  [SIGMOD Record](https://sigmodrecord.org/publications/sigmodRecord/0806/p28.helland.pdf)).
- **Los límites escasos no se resuelven solos.** Las estructuras que mezclan copias
  automáticamente (CRDT) no hacen cumplir por sí mismas un límite como "queda un solo
  asiento": eso necesita una compensación a nivel del negocio
  ([notas de CAP](https://www.cs.ubc.ca/~bestchai/teaching/cs538b_2020w1/slides/CAP.pdf)).
  Aquí el "asiento" es "un gafete / una persona adentro una sola vez".
- **El control de acceso con copia local tiene fecha de vencimiento.** Los lectores
  deciden con los permisos que tienen guardados: Kisi cubre cortes de hasta 36 horas
  ([Kisi](https://getkisi.com/updates/kisi-announcing-full-offline-support)) y MyQ deja
  configurar cuántas horas valen las credenciales guardadas
  ([MyQ](https://docs.myq-solution.com/en/deployment/v1/offline-login)).

## 4. Cómo queda en Limen

### 4.1 Dos clases de reglas

| Clase | Ejemplos | Sin conexión |
|---|---|---|
| **Locales:** se deciden con los datos de este equipo | formato de cédula y nombre, PRAIND vencido, acceso denegado, el gafete existe y está disponible según este equipo, este equipo ya tiene a la persona adentro | **Siempre se aplican**, igual que con conexión. Es lo que ya hace el núcleo |
| **Entre equipos:** dependen de lo último que hicieron los demás | la persona está adentro en otro equipo o sitio, el gafete se prestó en otro equipo, a la persona le negaron el acceso hace minutos | **No bloquean.** Se registra de forma provisional y se verifica al sincronizar |

Lo que antes bloqueaba (`SinVerificarEnLaNube`) pasa a ser **un aviso y una verificación
posterior**, no una puerta cerrada.

El reloj tampoco bloquea (regla E5): sin red, la hora sale del último ancla o del reloj
del equipo corregido con el último desfase medido, y si no se pudo comprobar el hecho
queda marcado como de hora no confiable. Al sincronizar, esa marca se suma al indicador
de incidente del registro.

### 4.2 El registro provisional

Todo hecho creado sin poder consultar a los demás queda marcado como **pendiente de
sincronizar**. Al sincronizar, o queda confirmado (nadie lo contradijo) o queda
**marcado como incidente**.

El registro **nunca se borra**. Es un hecho: la persona sí entró.

### 4.3 Al sincronizar: se detecta, no se rechaza

Las reglas entre equipos (A7, E1, un gafete prestado una vez) se comprueban al **unir**
los hechos de todos los equipos. Si se violan, no se descarta el segundo hecho ni se
falla el lote de sincronización: **se marca el registro con el indicador de incidente**.

Esto también significa que la unicidad de la nube (por ejemplo `presencia:⟨cédula⟩`)
no puede ser lo que rechace el envío: tiene que convertirse en esa marca. Se resuelve
cuando se construya la sincronización (hoja de ruta, paso 7).

### 4.4 El indicador de incidente (sin flujo aparte)

Se decidió mantenerlo simple: **no hay una tabla ni un flujo de incidencias**. Es una
marca en el propio registro (ingreso o préstamo de gafete) con:

- el **tipo** de incidente;
- cuándo se detectó;
- el registro con el que chocó (el otro ingreso, con su equipo, su operador y su hora);
- si ya fue **revisado**, por quién y con qué nota.

| Tipo | Qué pasó |
|---|---|
| Persona adentro dos veces | Dos equipos registraron la entrada de la misma persona sin saberlo (A7, E1) |
| Gafete prestado dos veces | Dos equipos entregaron el mismo número (F, K3) |
| Entró estando vetado | Se registró la entrada de alguien al que otro equipo le había negado el acceso (A8) |
| Salida duplicada | Dos salidas del mismo ingreso; gana la primera (E7), se anota |

**La causa habitual es una salida que alguien no registró.** Si una persona aparece
entrando dos veces, lo normal es que la primera visita nunca se cerró. Por eso un
incidente de este tipo **abre una revisión interna de quién no sacó a la persona**: la
pantalla muestra el ingreso anterior sin salida (quién lo registró, en qué equipo y a
qué hora) para saber a quién preguntarle.

**Cualquier operador puede marcar un incidente como revisado**, dejando una nota. Queda
auditado quién y cuándo (B11). Marcarlo como revisado **no borra ni cambia el hecho**;
sólo cierra la revisión. Si hay que corregir algo (por ejemplo cerrar la visita que
quedó abierta), se hace con el flujo normal, que también queda en la auditoría.

La lista de incidentes no es una entidad nueva: es una vista de los registros que
tienen el indicador y todavía no están revisados. En la lista "dentro" una persona
cuenta **una sola vez**, aunque tenga un incidente abierto.

### 4.4.1 Persona vetada que entró sin conexión

Es el único incidente que **avisa por sí solo**. Cuando se detecta (al sincronizar):

- **En el punto de acceso:** una alerta visible y persistente en el equipo que registró
  la entrada y en los demás equipos del punto, con el nombre de la persona y dónde
  está.
- **En el panel:** una alerta en vivo, además de aparecer en la lista de incidentes.

Los demás incidentes sólo aparecen en la lista y en el aviso de la pantalla.

### 4.5 Lo que ve el operador

- Un aviso fijo: "Sin conexión — N registros pendientes de sincronizar".
- Nunca un mensaje que le impida registrar por falta de red.
- Después de sincronizar, un aviso con la cantidad de incidentes sin revisar y la lista
  con el detalle del choque (equipo, hora, persona o gafete), para que no se pierda en
  una pestaña.

### 4.6 Qué tan viejos pueden ser los datos locales

La decisión local usa la última copia del catálogo (contratistas, accesos, gafetes) que
llegó a este equipo. Para no confiar en datos demasiado viejos:

- Se muestra desde cuándo es la copia ("datos de hace 3 horas").
- Pasado un umbral (propuesta: **24 horas**, alineado con el login sin conexión, regla
  L4) el aviso se vuelve más visible. **No bloquea**: la decisión final de dejar pasar
  es del guardia y el hecho queda registrado con esa advertencia.

## 5. Lo que no se hace

- **Bloquear un registro por falta de conexión** (el comportamiento de Lattis).
- **Borrar el registro conflictivo.** Se corrige con otro hecho.
- **Resolver en silencio** con "gana el más reciente". Una pérdida silenciosa en un
  registro de seguridad es peor que un incidente visible.
- **Adivinar quién tiene la razón.** El orden de llegada ayuda a sugerir cuál es el
  duplicado (el hecho más tardío), pero la decisión es de una persona.

## 6. Dónde cae en la hoja de ruta

Nada de esto cambia el núcleo actual, que ya decide con datos locales y no consulta a
nadie. Los hechos inmutables ya existen (regla E11, sección 9.1 de `arquitectura.md`):
cada entrada y salida queda en la tabla `hecho`, que la base no deja editar ni borrar.
Lo demás se construye junto con la sincronización (paso 7):

1. Estado de sincronización en cada hecho (pendiente, confirmado) y el indicador de
   incidente (tipo, registro con el que chocó, revisión) en ingresos y préstamos.
2. Un caso de uso para marcar un incidente como revisado, con nota (en `aplicacion`;
   la regla de qué tipos existen, en `dominio`).
3. Detección de las invariantes entre equipos al unir los hechos (en la nube).
4. Alerta de persona vetada: en los equipos del punto de acceso y en el panel (esto
   usa el aviso en vivo y las notificaciones, sección 10 de `arquitectura.md`).
5. Aviso de "sin conexión" y lista de incidentes en las pantallas.

Las reglas **E7**, **E8**, **E9** y **E10** de `reglas.md` describen este comportamiento.

## 7. Decisiones tomadas y por confirmar

**Decidido por el dueño:**

1. Cualquier operador puede marcar un incidente como revisado (sin roles especiales,
   regla L1). Queda auditado quién y cuándo.
2. Se mantiene simple: el registro se marca con un indicador de incidente, sin flujo ni
   entidad aparte. Un doble ingreso abre una revisión interna de quién no registró la
   salida.
3. Si entra alguien vetado sin conexión, se alerta en el punto de acceso y en el panel.

**Confirmado después:**

4. El aviso de datos viejos se activa a las **24 horas** (4.6).
5. No hay, por ahora, ninguna otra regla de Lattis que deba seguir bloqueando sin
   conexión. Si aparece una, se anota aquí con su motivo.
