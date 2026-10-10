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
  /** Si está perdido: quién lo debe (un contratista o una cédula). */
  deudor_contratista_id: string | null;
  deudor_cedula: string | null;
}

/** Un cambio de estado de un gafete. Al marcarlo perdido, un solo deudor. */
export interface CambioGafete {
  cambio: 'PERDIDO' | 'PAGADO' | 'APARECIDO' | 'DE_BAJA';
  deudor_contratista_id: string | null;
  deudor_cedula: string | null;
}
