import type { ColDef, ITooltipParams } from 'ag-grid-community';

/**
 * Tipos de columna de la app: una columna con `type: 'fecha'` o
 * `type: 'numero'` hereda el filtro que corresponde (antes, después, entre…
 * o mayor que, menor que…) en vez del de texto.
 */
export const tiposDeColumna: Record<string, ColDef> = {
  fecha: {
    filter: 'agDateColumnFilter',
    filterParams: {
      comparator: compararFechaYMD,
      inRangeFloatingFilterDateFormat: 'DD/MM/YYYY',
    },
  },
  numero: {
    filter: 'agNumberColumnFilter',
    filterParams: {},
  },
};

/**
 * Compara la fecha que eligió la persona con la de la celda, que viaja como
 * `AAAA-MM-DD`. Devuelve negativo, cero o positivo, como pide AG Grid.
 */
export function compararFechaYMD(elegida: Date, valorDeCelda: string | null): number {
  if (!valorDeCelda) {
    return -1;
  }
  const [anio, mes, dia] = valorDeCelda.slice(0, 10).split('-').map(Number);
  if (!anio || !mes || !dia) {
    return -1;
  }
  const celda = new Date(anio, mes - 1, dia);
  const dia_elegido = new Date(elegida.getFullYear(), elegida.getMonth(), elegida.getDate());
  return celda.getTime() - dia_elegido.getTime();
}

/** Texto completo al pasar el mouse, sólo para celdas de texto o número. */
export function textoDeTooltip(parametros: ITooltipParams): string | undefined {
  const valor: unknown = parametros.valueFormatted ?? parametros.value;
  return typeof valor === 'string' || typeof valor === 'number' ? String(valor) : undefined;
}

/** La clave de una columna: su `colId` o, si no lo tiene, su `field`. */
export function claveDeColumna<T>(columna: ColDef<T>): string | undefined {
  return columna.colId ?? (typeof columna.field === 'string' ? columna.field : undefined);
}
