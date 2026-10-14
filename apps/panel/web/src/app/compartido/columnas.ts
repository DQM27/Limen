import type { ColDef } from 'ag-grid-community';
import type { OperacionDeFiltro } from '../nucleo/nucleo';

/** Los filtros de texto de AG Grid y su código en el núcleo (regla N2). */
const OPERACIONES: Readonly<Record<string, OperacionDeFiltro>> = {
  contains: 'CONTIENE',
  notContains: 'NO_CONTIENE',
  equals: 'IGUAL',
  notEqual: 'DISTINTO',
  startsWith: 'EMPIEZA_CON',
  endsWith: 'TERMINA_CON',
};

/** El código que el núcleo entiende para una opción del filtro de AG Grid. */
export function operacionDeFiltro(opcion: string | null | undefined): OperacionDeFiltro {
  return (opcion && OPERACIONES[opcion]) || 'CONTIENE';
}

/** La clave de una columna: su `colId` o, si no lo tiene, su `field`. */
export function claveDeColumna<T>(columna: ColDef<T>): string | undefined {
  return columna.colId ?? (typeof columna.field === 'string' ? columna.field : undefined);
}
