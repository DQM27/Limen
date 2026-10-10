import type { ColumnState } from 'ag-grid-community';

/**
 * El diseño que cada persona le da a una tabla (qué columnas ve, en qué
 * orden, de qué ancho, cómo las ordenó y si muestra los filtros) se guarda en
 * este equipo, por tabla, para que al volver la encuentre como la dejó.
 *
 * Es una comodidad: si el almacenamiento no está disponible (modo privado,
 * cuota llena) o lo guardado no sirve, la tabla arranca con su diseño de
 * siempre y nada se rompe.
 */
export interface DisenoGuardado {
  /** Columnas ocultas (por `colId` o `field`). */
  ocultas: string[];
  /** Orden, ancho, ordenamiento y fijado de cada columna. */
  columnas: ColumnState[];
  filtrosVisibles: boolean;
}

export function claveDeDiseno(id: string): string {
  return `limen.tabla.${id}`;
}

export function leerDiseno(id: string | undefined): DisenoGuardado | null {
  if (!id) {
    return null;
  }
  try {
    const texto = localStorage.getItem(claveDeDiseno(id));
    if (!texto) {
      return null;
    }
    const valor: unknown = JSON.parse(texto);
    if (typeof valor !== 'object' || valor === null) {
      return null;
    }
    const candidato = valor as Partial<DisenoGuardado>;
    return {
      ocultas: Array.isArray(candidato.ocultas)
        ? candidato.ocultas.filter((clave) => typeof clave === 'string')
        : [],
      columnas: Array.isArray(candidato.columnas) ? candidato.columnas : [],
      filtrosVisibles:
        typeof candidato.filtrosVisibles === 'boolean' ? candidato.filtrosVisibles : true,
    };
  } catch {
    return null;
  }
}

export function guardarDiseno(id: string | undefined, diseno: DisenoGuardado): void {
  if (!id) {
    return;
  }
  try {
    localStorage.setItem(claveDeDiseno(id), JSON.stringify(diseno));
  } catch {
    // Perder el diseño guardado no es motivo para romper la tabla.
  }
}
