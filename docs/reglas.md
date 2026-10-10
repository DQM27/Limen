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
| B3 | Todo contratista pertenece a una empresa que existe. | ✅ |
| B4 | Tipos de ingreso: sólo PRAIND e IN HOUSE. No existen SWAT, POR CORREO ni personal de ruta. | ✅ |
| B5 | El PRAIND (cursos de seguridad, con vencimiento) es obligatorio para todos. | ✅ |
| B6 | No se registra un contratista con el PRAIND vencido. Vencer hoy todavía es vigente. | ✅ |
| B8 | Al editar, el vencimiento sólo se revisa si cambió la fecha (para poder quitar el acceso o corregir el nombre de alguien con el PRAIND vencido). | ✅ |
| B9 | No se cambia la cédula de alguien que está adentro. | ✅ |
| B10 | Sólo PRAIND recibe gafete físico; para IN HOUSE su credencial ya es el gafete. | ✅ |
| B11 | Todo cambio queda en la auditoría, en todos los campos, con el antes y el después. La auditoría no se edita ni se borra (lo impide la base). | ✅ |
| B12 | Contratistas y empresas son globales: un cambio se ve en todos los sitios. | ⏳ |

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
| E2 | En vehículo la placa es obligatoria (mayúsculas; letras, números, espacios y guiones; hasta 20); a pie lo escrito se descarta. | ✅ |
| E3 | Si lleva gafete, debe existir en el catálogo, estar disponible y no estar prestado. El gafete es opcional ("sin gafete") aunque el tipo lo requiera; a IN HOUSE se le ignora el número. | ✅ |
| E4 | La salida no puede ser anterior a la entrada, y un ingreso no se cierra dos veces. | ✅ |
| E5 | Si el reloj del equipo retrocedió respecto al último movimiento, no se registran movimientos. | ✅ |
| E6 | El movimiento lo registra una sesión válida y activa. | ⏳ |
| E7 | Ante una salida duplicada desde dos equipos, gana la primera. | ⏳ |
| E8 | **Nunca se bloquea un registro por falta de red.** Sin conexión se registra de forma provisional con las reglas locales; al sincronizar, si hubo choque entre equipos (E1, gafete prestado dos veces, entró estando vetado), el registro **no se borra**: queda marcado como incidente. Ver [`operacion-sin-conexion.md`](operacion-sin-conexion.md). | ⏳ |
| E9 | Un registro en conflicto queda marcado con un indicador de incidente (sin flujo ni tabla aparte). Cualquier operador puede marcarlo como revisado dejando una nota, auditado. Un doble ingreso abre una revisión interna de quién no registró la salida anterior. Nunca se borra el hecho. | ⏳ |
| E10 | Si alguien vetado (A8) entra mientras el equipo estaba sin conexión, al sincronizar se alerta en el punto de acceso y en el panel. | ⏳ |
| E11 | Cada entrada y salida (las cuatro vías; para el personal KOF, la entrega y la devolución del provisional) queda como un **hecho** que nunca se edita ni se borra. Se guarda junto con el movimiento o no se guarda ninguno; lo que quedó mal se corrige con otro hecho. | ✅ |

## F. Gafetes

| Código | Regla | Estado |
|---|---|---|
| F1 | Número mayor a cero, sin repetir dentro de su tipo; se pueden crear por rango (hasta 1.000 de una vez). | ✅ |
| F2 | Tipos: contratista, visita (la usa el ingreso por correo), proveedor y provisional KOF. | ✅ |
| F3 | Estados: Disponible → Perdido → Disponible (pagado o apareció, queda en la auditoría), y Disponible → De baja. | ✅ |
| F4 | Sólo un gafete disponible se da de baja o se marca perdido. | ✅ |
| F5 | Marcar perdido exige indicar su **último portador** (quién lo tenía), que corresponde al tipo del gafete: un contratista para el de contratista, un proveedor o una visita (por su cédula, que debe ser nacional o de extranjero, A3) para el de proveedor o de visita, y alguien del personal KOF para el provisional. Como en Lattis. | ✅ |
| F6 | No se da de baja un gafete prestado en este momento. | ✅ |

## H. Proveedores

| Código | Regla | Estado |
|---|---|---|
| H1 | La empresa proveedora existe; su nombre es obligatorio y no se repite. Es un catálogo aparte de las empresas de contratistas. | ✅ |
| H2 | Cédula según A3, nombre obligatorio (sin catálogo de personas: se toman en cada ingreso); aplican A7 y A8. | ✅ |
| H3 | No puede tener dos ingresos abiertos. El gafete de proveedor es obligatorio y debe poder prestarse (E3); salida y reloj como E4–E5. | ✅ |

## I. Ingreso por correo

| Código | Regla | Estado |
|---|---|---|
| I1 | Cédula según A3, nombre y motivo obligatorios (motivo: texto libre, hasta 200 caracteres); aplican A7 y A8. | ✅ |
| I2 | No puede tener dos ingresos abiertos. El gafete de visita es obligatorio y debe poder prestarse (E3); salida y reloj como E4–E5. | ✅ |

## K. Personal KOF (personal interno de FEMSA)

| Código | Regla | Estado |
|---|---|---|
| K1 | El gafete provisional existe en el inventario KOF y está disponible. | ✅ |
| K2 | La persona se identifica por código de empleado (5 a 7 dígitos, único) y debe estar activa. Se desactiva, no se borra. | ✅ |
| K3 | Un provisional por persona a la vez, y un número prestado a una sola persona. Entregar el gafete provisional es la entrada (queda en la presencia y mueve el reloj) y devolverlo es la salida. | ✅ |

## L. Usuarios

| Código | Regla | Estado |
|---|---|---|
| L1 | Un solo rol: Operador. Sin roles especiales. | ⏳ |
| L2 | Los usuarios se crean en el panel y se desactivan, no se borran. | ⏳ |
| L3 | Contraseña de mínimo 8 caracteres. | ⏳ |
| L4 | Login sin conexión con credencial guardada, por un máximo de 24 horas. | ⏳ |
| L5 | Sesión única por equipo; cerrar la app cierra la sesión en la nube. | ⏳ |

## M. Equipos

| Código | Regla | Estado |
|---|---|---|
| M1 | Cada equipo se vincula con un código de un solo uso y su propio par de claves. | ⏳ |
| M2 | Un equipo retirado pierde el acceso al instante y se cierra su sesión. | ⏳ |
| M3 | Tipos: PC, celular y visor; el visor sólo lee. | ⏳ |

## Fuera de alcance por ahora

Visitas con cita agendada y salidas de rutas: no están aprobadas del todo y no se
incluyen hasta que lo estén.
