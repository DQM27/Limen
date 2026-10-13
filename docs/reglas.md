# Reglas de negocio

Catálogo de las reglas acordadas para Limen, a partir de las que estaban en producción
en Lattis. Cada regla tiene un código estable para poder referirse a ella.

Estado: ✅ implementada en `crates/dominio` · ⏳ acordada, pendiente de implementar.

## A. Identidad de personas

| Código | Regla | Estado |
|---|---|---|
| A1 | La cédula se normaliza: sin espacios, guiones ni puntos; en mayúsculas; 10 dígitos que empiezan en 0 (formato del TSE) pierden ese cero. | ✅ |
| A2 | Cualquier documento: máximo 20 caracteres, sólo letras y números. | ✅ |
| A3 | Contratistas, proveedores y correo: sólo cédula nacional o de extranjero, números de 9 a 13 dígitos. | ✅ |
| A5 | Nombre de persona: sólo letras de la A a la Z, la Ñ y espacios. Las tildes y la diéresis se quitan solas ("José" → "JOSE"); números y símbolos (incluidos apóstrofo y guion) se rechazan. | ✅ |
| A6 | Todo nombre de persona o empresa se guarda en MAYÚSCULAS y sin espacios de más. | ✅ |
| A7 | Una persona sólo puede estar adentro por una vía a la vez (contratista, proveedor, correo o KOF). El personal KOF se identifica por código de empleado, no por cédula. | ✅ las cuatro vías. El personal KOF está adentro mientras tenga un gafete provisional sin devolver (se le reconoce por el código de empleado). |
| A8 | Veto por persona: una cédula con acceso denegado se rechaza por cualquier vía. | ✅ |

## B. Contratistas

| Código | Regla | Estado |
|---|---|---|
| B1 | Cédula y nombre obligatorios (A1–A6). | ✅ |
| B2 | La cédula no se repite. | ✅ |
| B3 | Todo contratista pertenece a una empresa que existe. Sin empresa elegida es `empresa_requerida` ("La empresa es obligatoria"); con una que no existe, `empresa_no_existe`. | ✅ |
| B4 | Tipos de ingreso: sólo PRAIND e IN HOUSE. No existen SWAT, POR CORREO ni personal de ruta. | ✅ |
| B5 | El PRAIND (cursos de seguridad, con vencimiento) es obligatorio para todos. Sin fecha es `praind_requerido`. | ✅ |
| B5b | El formulario del contratista informa **todos** los problemas a la vez, cada uno con su campo (`contratista::problemas`, en el orden de `ErrorContratista`); registrar y editar informan el primero. El caso de uso `ValidarContratista` lee lo que hace falta (cédula en uso, empresa existente, si está adentro) y el dominio decide. | ✅ |
| B5c | Mientras se escribe, el nombre de una persona se ve como quedará (mayúsculas y sin tildes: `nombre_mientras_se_escribe`) y el de una empresa en mayúsculas (`nombre_empresa_mientras_se_escribe`); la forma la da el dominio. | ✅ |
| B6 | No se registra un contratista con el PRAIND vencido. Vencer hoy todavía es vigente. | ✅ |
| B8 | Al editar, el vencimiento sólo se revisa si cambió la fecha (para poder quitar el acceso o corregir el nombre de alguien con el PRAIND vencido). | ✅ |
| B9 | No se cambia la cédula de alguien que está adentro. | ✅ |
| B10 | Sólo PRAIND recibe gafete físico; para IN HOUSE su credencial ya es el gafete. | ✅ |
| B11 | Todo cambio queda en la auditoría, en todos los campos, con el antes y el después. La auditoría no se edita ni se borra (lo impide la base). | ✅ |
| B12 | Contratistas y empresas son globales: un cambio se ve en todos los sitios. | ✅ con la sincronización en tiempo real (`nube.md`, secciones 9 y 10). |

## C. Empresas

| Código | Regla | Estado |
|---|---|---|
| C1 | Nombre obligatorio, sin repetir. Admite números y signos ("ACME S.A."), hasta 150 caracteres. | ✅ |

Una empresa no se bloquea ni se desactiva: si ya no tiene trabajadores, nadie entra por ella.

## D. Acceso al entrar (contratista)

En este orden; la primera que aplica decide:

| Código | Regla | Estado |
|---|---|---|
| D2 | Sin acceso → "acceso denegado para esta persona". Tiene prioridad sobre el PRAIND. | ✅ |
| D4 | PRAIND vencido → denegado. | ✅ |
| D5 | PRAIND que vence en 30 días o menos → entra con advertencia. | ✅ |

No se guarda una "versión de reglas" en cada ingreso: la auditoría (B11) permite
reconstruir cómo estaba el contratista el día que entró.

## E. Ingreso y salida

| Código | Regla | Estado |
|---|---|---|
| E1 | No puede tener dos ingresos abiertos. "Ya está adentro" pesa más que el acceso denegado: lo que corresponde es registrar la salida. (Entre equipos y sitios: fase de nube.) | ✅ |
| E2 | En vehículo la placa es obligatoria (mayúsculas; letras, números, espacios y guiones; hasta 20); a pie lo escrito se descarta. Mientras se escribe se ve en mayúsculas (`medio::placa_mientras_se_escribe`), como el nombre. | ✅ |
| E3 | A quien le corresponde gafete (PRAIND) el operador le indica **un número o «Sin gafete» (S/G)**, a propósito: sin ninguno de los dos no entra (`gafete_requerido`), así nunca queda S/G por olvido. Con número, el gafete debe existir en el catálogo, estar disponible y no estar prestado. El S/G no pide motivo (el hecho dice qué operador lo registró) y no se le asigna gafete después (quien entra S/G normalmente sale y no regresa). A IN HOUSE no le aplica: se ignora lo indicado. El ingreso guarda qué pasó: prestado, S/G o no aplica. | ✅ |
| E4 | La salida no puede ser anterior a la entrada, y un ingreso no se cierra dos veces. | ✅ |
| E5 | El reloj nunca detiene la portería. Cada movimiento se sella con la hora de una fuente externa (NTP; la nube cuando exista) anclada al reloj monotónico del proceso, que nadie puede mover. Si la hora quedó antes del último movimiento (el reloj retrocedió) se sella con la de ese movimiento, para que el historial nunca vaya hacia atrás. La hora es **no confiable** si retrocedió, si no se pudo comprobar contra la fuente o si su margen de error pasa de 60 s: el movimiento se registra igual y el hecho queda marcado, junto con la hora cruda del reloj del equipo (un salto entre las dos delata un reloj cambiado a mano). | ✅ |
| E6 | El movimiento lo registra una sesión válida y activa. | ⏳ |
| E7 | Ante una salida duplicada desde dos equipos, gana la primera. | ⏳ |
| E8 | **Nunca se bloquea un registro por falta de red.** Sin conexión se registra de forma provisional con las reglas locales; al sincronizar, si hubo choque entre equipos (E1, gafete prestado dos veces, entró estando vetado), el registro **no se borra**: queda marcado como incidente. Ver [`operacion-sin-conexion.md`](operacion-sin-conexion.md). | ⏳ |
| E9 | Un registro en conflicto queda marcado con un indicador de incidente (sin flujo ni tabla aparte). Cualquier operador puede marcarlo como revisado dejando una nota, auditado. Un doble ingreso abre una revisión interna de quién no registró la salida anterior. Nunca se borra el hecho. | ⏳ |
| E10 | Si alguien vetado (A8) entra mientras el equipo estaba sin conexión, al sincronizar se alerta en el punto de acceso y en el panel. | ⏳ |
| E11 | Cada entrada y salida (las cuatro vías; para el personal KOF, la entrega y la devolución del provisional) queda como un **hecho** que nunca se edita ni se borra. Se guarda junto con el movimiento o no se guarda ninguno; lo que quedó mal se corrige con otro hecho. | ✅ |
| E12 | Cada marca (entrada y salida; para el personal KOF, entrega y devolución) guarda **su propio responsable** y su instante: quien registra la salida puede ser otro operador que el de la entrada. "Dentro" y el historial muestran los dos con el nombre del usuario; si ese usuario no está en el equipo, queda su ID. El instante se guarda como uno solo (fecha y hora juntas, UTC, para que ordene bien y no se descuadre a medianoche); la pantalla lo muestra en columnas separadas de fecha y hora, en la hora de Costa Rica. | ✅ |

| E13 | Quien lleva **más de 12 horas** adentro (exactamente 12 todavía no) se marca para revisión, porque casi siempre falta registrar su salida (`permanencia::lleva_demasiado_adentro`). "Dentro" lo resalta; sólo marca: no cierra el ingreso ni bloquea nada. Lo decide el dominio, no la pantalla. | ✅ |

## F. Gafetes

| Código | Regla | Estado |
|---|---|---|
| F1 | Número mayor a cero, sin repetir dentro de su tipo; se pueden crear por rango (hasta 1.000 de una vez). | ✅ |
| F2 | Tipos: contratista, visita (la usa el ingreso por correo), proveedor y provisional KOF. | ✅ |
| F3 | Estados: Disponible → Perdido → Disponible (pagado o apareció, queda en la auditoría), y Disponible → De baja. | ✅ |
| F4 | Sólo un gafete disponible se da de baja o se marca perdido. | ✅ |
| F5 | Marcar perdido exige indicar su **último portador** (quién lo tenía), que corresponde al tipo del gafete: un contratista para el de contratista, un proveedor o una visita (por su cédula, que debe ser nacional o de extranjero, A3) para el de proveedor o de visita, y alguien del personal KOF para el provisional. Como en Lattis. | ✅ |
| F6 | No se da de baja un gafete prestado en este momento. | ✅ |
| F7 | El listado de gafetes dice **a quién está asignado** cada uno: quien lo tiene prestado ahora (sale de quién está adentro por la vía de ese tipo, F2: `TipoGafete::via`) o, si está perdido, su último portador, con su nombre si está en un catálogo (contratista, personal KOF) y sólo su cédula si es un proveedor o una visita. Quién lo tiene prestado lo decide `dominio/gafete_prestado.rs` (`quienes_llevan`). | ✅ |
| F8 | **Salida por número de gafete:** se busca a quien lo devuelve entre los que están adentro (en la vía de la pantalla o en las cuatro). Si nadie lo tiene, se rechaza (`gafete_sin_portador`). Cada tipo tiene su numeración, así que un número puede estar en uso en más de una vía a la vez: entonces no se adivina, se rechaza (`gafete_en_varias_vias`, nombrando las vías) y la salida se registra desde la fila de la persona. | ✅ |

## H. Proveedores

| Código | Regla | Estado |
|---|---|---|
| H1 | La empresa proveedora existe; su nombre es obligatorio y no se repite. Es un catálogo aparte de las empresas de contratistas. | ✅ |
| H2 | Cédula según A3, nombre obligatorio (sin catálogo de personas: se toman en cada ingreso); aplican A7 y A8. | ✅ |
| H3 | No puede tener dos ingresos abiertos. El gafete de proveedor es obligatorio y debe poder prestarse (E3); salida y reloj como E4–E5. | ✅ |
| H4 | Como B5b: el formulario del ingreso informa **todos** los problemas a la vez, cada uno con su campo (`ingreso_proveedor::problemas`: persona, medio, empresa —obligatoria y existente—, ya adentro, veto, gafete); registrar informa el primero. La empresa vacía es `empresa_proveedora_requerida`. El caso de uso `ValidarEntradaProveedor` lee lo que hace falta y el dominio decide. | ✅ |

## I. Ingreso por correo

| Código | Regla | Estado |
|---|---|---|
| I1 | Cédula según A3, nombre y motivo obligatorios (motivo: texto libre, hasta 200 caracteres); aplican A7 y A8. | ✅ |
| I2 | No puede tener dos ingresos abiertos. El gafete de visita es obligatorio y debe poder prestarse (E3); salida y reloj como E4–E5. | ✅ |
| I3 | Como B5b: el formulario del ingreso informa **todos** los problemas a la vez, cada uno con su campo (`ingreso_correo::problemas`: persona, motivo, medio, ya adentro, veto, gafete); registrar informa el primero. El caso de uso `ValidarEntradaCorreo` lee lo que hace falta y el dominio decide. | ✅ |

## K. Personal KOF (personal interno de FEMSA)

| Código | Regla | Estado |
|---|---|---|
| K1 | El gafete provisional existe en el inventario KOF y está disponible. | ✅ |
| K2 | La persona se identifica por código de empleado (5 a 7 dígitos, único) y debe estar activa. Se desactiva, no se borra. | ✅ |
| K3 | Un provisional por persona a la vez, y un número prestado a una sola persona. Entregar el gafete provisional es la entrada (queda en la presencia y mueve el reloj) y devolverlo es la salida. | ✅ |
| K4 | Al entregar el gafete hay que elegir a la persona: sin elegirla el error es `persona_requerida` ("Elija a la persona del personal KOF"), no un identificador ilegible. | ✅ |
| K5 | Como B5b: el alta de personal (`personal_kof::problemas`: código, nombre y código repetido) y la entrega del gafete (`prestamo_kof::problemas_de_entrega`: persona elegida, activa y sin otro provisional, y gafete) informan **todos** sus problemas a la vez, cada uno con su campo; registrar y entregar informan el primero. Casos de uso `ValidarPersonalKof` y `ValidarEntregaKof`. | ✅ |

## L. Usuarios

| Código | Regla | Estado |
|---|---|---|
| L1 | Un solo rol: Operador. Sin roles especiales. Cada hecho y cada cambio queda a nombre del usuario que tenía la sesión. | ✅ |
| L2 | Los usuarios son **globales: se crean y se administran en la nube**, nunca en un equipo. Se desactivan, no se borran, y nadie se desactiva a sí mismo. La cédula es su identidad para entrar: nacional o de extranjero (A1 y A3), única y no se edita. En el equipo sólo se inicia sesión, se cambia la propia clave (pidiendo la actual) y se cierra la sesión. | ✅ |
| L3 | Clave de 8 a 128 caracteres que no puede ser la propia cédula. Se guarda sólo su hash Argon2id (sal aleatoria, formato PHC); nunca va a la auditoría ni a la interfaz. | ✅ |
| L4 | Login sin conexión con credencial guardada, por un máximo de 24 horas. | ⏳ |
| L5 | Sesión única por equipo; cerrar la app cierra la sesión en la nube. | ⏳ |
| L6 | Tras 5 intentos fallidos seguidos con la misma cédula, se bloquea su inicio de sesión por 5 minutos (aunque después escriba la clave correcta). Un fallo de hace más de 5 minutos ya no suma; entrar borra la cuenta. Las cédulas que no existen también se bloquean. | ✅ |
| L7 | El inicio de sesión no revela qué cédulas existen: cédula desconocida o clave equivocada dan el mismo error y tardan lo mismo; "desactivado" sólo se le dice a quien escribió la clave correcta. | ✅ |
| L8 | Un equipo no crea usuarios: le llegan de la nube. Si la nube le puso a alguien una clave temporal, al entrar sólo puede cambiarla o cerrar la sesión. Al desarrollar (compilación de depuración), mientras no exista la nube, la base se siembra con usuarios de prueba; la versión para producción no trae esa semilla. | ✅ |

## M. Equipos

| Código | Regla | Estado |
|---|---|---|
| M1 | Cada equipo se vincula con un código de un solo uso y su propio par de claves. | ⏳ |
| M2 | Un equipo retirado pierde el acceso al instante y se cierra su sesión. | ⏳ |
| M3 | Tipos: PC, celular y visor; el visor sólo lee. | ⏳ |
| M4 | Cada equipo pertenece a una **unidad operativa** (el "sitio" de Lattis). Los catálogos (contratistas, empresas, empresas proveedoras, personal KOF, usuarios) son **globales**: iguales en todas las unidades. Gafetes, ingresos, presencia, préstamos e historial son **de la unidad**: un equipo baja sólo los de la suya. Ver `nube.md`. | ✅ en el equipo: cada hecho lleva su unidad y su equipo, y lo de la unidad lleva su campo `unidad` (los gafetes, también en la clave). El reparto llega con la sincronización. |
| M5 | El equipo es desechable: la nube es la fuente de verdad. Guarda una copia local y trabaja sin conexión; lo pendiente se sube al volver la red. | ✅ subida y bajada en tiempo real, carga inicial de un equipo nuevo (`nube.md`). |
| M6 | El administrador **registra** un equipo desde el panel y recibe un código de 10 caracteres (sin 0, 1, I ni O) que vale 15 minutos y sirve **una sola vez**. | ✅ en el dominio (`vinculacion.rs`); falta el Worker y el panel. |
| M7 | Sólo hay dos acciones sobre un equipo: **registrar** y **retirar**. Un equipo reinstalado se registra como nuevo y el anterior se retira. | ⏳ |
| M8 | El código se guarda sólo como huella (SHA-256) y no se imprime en los registros técnicos. Un código usado, anulado o vencido deja un evento de seguridad. | ✅ la regla; ⏳ el evento. |

## N. Buscador

| Código | Regla | Estado |
|---|---|---|
| N1 | Al buscar, lo que coincide se **resalta** en los resultados con la misma regla del buscador: sin importar mayúsculas ni tildes (la Ñ cuenta como N), una palabra completa se resalta entera; el comienzo de una palabra o un fragmento dentro de ella, sólo lo escrito; con errores de tecleo, la palabra parecida entera. Cada palabra escrita se resalta donde aparece; la que no aparece en ese texto no marca nada (qué filas coinciden lo decide el buscador o el filtro, y una búsqueda puede repartir sus palabras entre columnas: nombre y empresa). La cédula se resalta ignorando guiones, puntos y espacios. El dominio entrega el texto partido en segmentos (`dominio/resaltado.rs`); la pantalla sólo los pinta. | ✅ |
| N2 | El buscador de una tabla y sus filtros de texto por columna se deciden en el núcleo; la pantalla sólo pregunta y aplica la respuesta, sin repetir ninguna regla. **Buscador:** cada palabra escrita debe aparecer en alguna columna de la fila, en cualquier orden, con las reglas del buscador (palabra completa, comienzo, dentro de una palabra o con errores de tecleo); si son sólo números, busca por cédula o código y también dentro de la empresa; vacío no excluye a nadie. Es la misma coincidencia que usa el resaltado (N1). **Filtros de columna:** sin importar mayúsculas ni tildes (la Ñ cuenta como N); «contiene» parte por palabras en cualquier orden y «no contiene» es su contrario; «igual», «distinto», «empieza con» y «termina con» comparan una frase; sin texto de filtro todo coincide. | ✅ |

## Fuera de alcance por ahora

Visitas con cita agendada y salidas de rutas: no están aprobadas del todo y no se
incluyen hasta que lo estén.
