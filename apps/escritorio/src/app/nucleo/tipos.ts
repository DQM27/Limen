/**
 * Lo que devuelve el núcleo, tal como viaja en JSON (ver `dto.rs` en
 * `apps/escritorio/comandos`). Es sólo la forma de los datos: la interfaz no
 * decide nada con ellos.
 *
 * Identificadores como texto (UUID), fechas `AAAA-MM-DD`, instantes RFC 3339
 * en UTC, y los tipos con su código estable en mayúsculas.
 */

export type TipoIngreso = 'PRAIND' | 'IN_HOUSE';

/** Lo que el dominio decidió hoy sobre el acceso de un contratista (regla D). */
export interface Acceso {
  resultado: 'PERMITIDO' | 'PERMITIDO_CON_ADVERTENCIA' | 'DENEGADO';
  /** Con advertencia: cuántos días le quedan al PRAIND (0 = vence hoy). */
  dias_para_vencer: number | null;
  /** Denegado: el código del motivo (`sin_acceso`, `praind_vencido`). */
  motivo: string | null;
}

/** Una fila de la grilla de contratistas. */
export interface FilaContratista {
  id: string;
  cedula: string;
  nombre: string;
  empresa_id: string;
  empresa_nombre: string;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string;
  tiene_acceso: boolean;
  requiere_gafete: boolean;
  acceso: Acceso;
}

/** Una empresa de contratistas. */
export interface Empresa {
  id: string;
  nombre: string;
}

/**
 * El formulario de un contratista, tal cual lo escribe el operador: el
 * núcleo normaliza y valida (cédula, nombre, PRAIND). Las claves son las que
 * trae `ErrorApp.campo` cuando un dato no pasa.
 */
export interface ContratistaEntrada {
  cedula: string;
  nombre: string;
  empresa_id: string;
  tipo_ingreso: TipoIngreso;
  /** `AAAA-MM-DD`. */
  fecha_vencimiento_praind: string;
  tiene_acceso: boolean;
}

/** Un campo que cambió al editar, con el antes y el después (regla B11). */
export interface Cambio {
  campo: string;
  antes: string;
  despues: string;
}

/** Una empresa proveedora (catálogo aparte del de contratistas). */
export interface EmpresaProveedora {
  id: string;
  nombre: string;
}

export type Medio = 'A_PIE' | 'VEHICULO';

/** El formulario de entrada de un proveedor. */
export interface EntradaProveedor {
  cedula: string;
  nombre: string;
  empresa_id: string;
  medio: Medio;
  /** Obligatoria en vehículo; a pie se descarta. */
  placa: string | null;
  gafete: number;
}

/** El formulario de un ingreso por correo (visita autorizada). */
export interface EntradaCorreo {
  cedula: string;
  nombre: string;
  motivo: string;
  medio: Medio;
  placa: string | null;
  gafete: number;
}

/** Una persona del personal KOF. */
export interface PersonalKof {
  id: string;
  codigo_empleado: string;
  nombre: string;
  activo: boolean;
}

export type TipoGafete = 'CONTRATISTA' | 'VISITA' | 'PROVEEDOR' | 'PROVISIONAL_KOF';

/** Un gafete del catálogo, con su estado y si está prestado ahora. */
export interface Gafete {
  tipo: TipoGafete;
  numero: number;
  estado: 'DISPONIBLE' | 'PERDIDO' | 'DE_BAJA';
  prestado: boolean;
  /**
   * Si está perdido, su último portador: el campo de su clase trae valor
   * (contratista, cédula de un proveedor o una visita, o personal KOF).
   */
  portador_contratista_id: string | null;
  portador_cedula: string | null;
  portador_personal_id: string | null;
}

/**
 * Un cambio de estado de un gafete. Al marcarlo perdido, un solo último
 * portador; que corresponda al tipo del gafete lo decide el núcleo.
 */
export interface CambioGafete {
  cambio: 'PERDIDO' | 'PAGADO' | 'APARECIDO' | 'DE_BAJA';
  portador_contratista_id: string | null;
  portador_cedula: string | null;
  portador_personal_id: string | null;
}

export type Via = 'CONTRATISTA' | 'PROVEEDOR' | 'CORREO' | 'KOF';

/** Un contratista en el buscador o en la ficha del ingreso: todo decidido. */
export interface CandidatoIngreso {
  id: string;
  cedula: string;
  nombre: string;
  empresa_id: string;
  empresa_nombre: string | null;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string;
  tiene_acceso: boolean;
  /** PRAIND: hay que indicar un gafete o «Sin gafete». IN HOUSE: no aplica. */
  requiere_gafete: boolean;
  puede_entrar: boolean;
  /** Si puede: permitido, o con aviso de PRAIND por vencer. */
  acceso: Acceso | null;
  /** Si no puede: el mismo motivo que daría registrar. */
  motivo: { codigo: string; mensaje: string } | null;
  /** Si ya está adentro: por qué vía, para ofrecer registrar su salida. */
  adentro_por: Via | null;
  /** Gafetes de contratista perdidos a su nombre: sólo informa. */
  gafetes_perdidos: number[];
}

/** El formulario de entrada de un contratista. */
export interface EntradaContratista {
  contratista_id: string;
  medio: Medio;
  placa: string | null;
  /** El número de gafete, o `null` con `sin_gafete` marcado (o si no aplica). */
  gafete: number | null;
  /** «Sin gafete» (S/G), marcado a propósito. */
  sin_gafete: boolean;
}

/** La entrada registrada y el resultado del acceso (con su aviso, si hay). */
export interface EntradaRegistrada {
  ingreso_id: string;
  acceso: Acceso;
}

/** Una persona que está adentro, por cualquiera de las cuatro vías. */
export interface PersonaAdentro {
  via: Via;
  /** El ingreso abierto: con él se registra la salida. */
  ingreso_id: string;
  /** La cédula o, para el personal KOF, el código de empleado. */
  identidad: string;
  nombre: string;
  procedencia: string;
  medio: Medio | null;
  placa: string | null;
  gafete: number | null;
  /** Entró sin gafete (S/G): se muestra «S/G». */
  sin_gafete: boolean;
  /** Cuándo entró: fecha y hora juntas (RFC 3339, UTC). */
  entrada: string;
  /** Quién registró la entrada. */
  entrada_por: Operador;
}

/** Quién registró una marca (entrada o salida). */
export interface Operador {
  id: string;
  /** `null` si ese usuario no está en este equipo. */
  nombre: string | null;
}

// --- Usuarios e inicio de sesión (bloque L) ---

/** Quién tiene la sesión en este equipo. */
export interface UsuarioActual {
  id: string;
  cedula: string;
  nombre: string;
  /** Clave temporal: hay que cambiarla antes de hacer cualquier otra cosa. */
  debe_cambiar_clave: boolean;
}

/** Un usuario en la lista de usuarios (nunca trae la clave). */
export interface Usuario {
  id: string;
  cedula: string;
  nombre: string;
  /** Se desactiva, no se borra. */
  activo: boolean;
  debe_cambiar_clave: boolean;
}

/**
 * El formulario de alta de un usuario. Para el primer usuario del equipo la
 * clave es la suya; para los demás, una temporal.
 */
export interface UsuarioEntrada {
  cedula: string;
  nombre: string;
  clave: string;
}

// --- Historial de ingresos ---

/** Una fila del historial: una entrada, con su salida si ya salió. */
export interface Movimiento {
  via: Via;
  ingreso_id: string;
  /** La cédula o, para el personal KOF, el código de empleado. */
  identidad: string;
  nombre: string;
  procedencia: string;
  medio: 'A_PIE' | 'VEHICULO' | null;
  placa: string | null;
  gafete: number | null;
  /** Entró sin gafete: se muestra «S/G». */
  sin_gafete: boolean;
  /** Fecha y hora juntas (RFC 3339, UTC). */
  entrada: string;
  /** Quién registró la entrada. */
  entrada_por: Operador;
  /** `null` mientras siga adentro. */
  salida: string | null;
  /** Quién registró la salida; puede ser otro operador que el de la entrada. */
  salida_por: Operador | null;
}

/** El historial de un rango de fechas. */
export interface Historial {
  /** `AAAA-MM-DD`; `null` = sin límite. */
  desde: string | null;
  hasta: string | null;
  /** Del más reciente al más antiguo. */
  movimientos: Movimiento[];
  /** Había más de `maximo` movimientos: sólo vienen los más recientes. */
  truncado: boolean;
  maximo: number;
}

/** Un acceso rápido de fecha (hoy, esta semana…) con su rango de hoy. */
export interface AtajoFecha {
  codigo: string;
  /** Para el menú. */
  etiqueta: string;
  /** Para el botón. */
  corta: string;
  desde: string | null;
  hasta: string | null;
  /** El que abre el historial. */
  por_omision: boolean;
}
