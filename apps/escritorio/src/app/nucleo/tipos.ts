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
