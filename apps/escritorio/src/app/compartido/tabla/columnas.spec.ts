import type { ITooltipParams } from 'ag-grid-community';
import { claveDeColumna, compararFechaYMD, textoDeTooltip, tiposDeColumna } from './columnas';

describe('columnas de la tabla', () => {
  it('compararFechaYMD ordena la celda respecto del día elegido', () => {
    const elegida = new Date(2026, 9, 9); // 9 de octubre de 2026
    expect(compararFechaYMD(elegida, '2026-10-09')).toBe(0);
    expect(compararFechaYMD(elegida, '2026-10-08')).toBeLessThan(0);
    expect(compararFechaYMD(elegida, '2026-10-10')).toBeGreaterThan(0);
    expect(compararFechaYMD(elegida, '2026-10-09T23:59:00Z')).toBe(0);
  });

  it('una celda sin fecha, o con una que no se entiende, queda antes de cualquier día', () => {
    const elegida = new Date(2026, 9, 9);
    expect(compararFechaYMD(elegida, null)).toBe(-1);
    expect(compararFechaYMD(elegida, 'no es fecha')).toBe(-1);
  });

  it('los tipos fecha y número traen su filtro propio', () => {
    expect(tiposDeColumna['fecha']?.filter).toBe('agDateColumnFilter');
    expect(tiposDeColumna['numero']?.filter).toBe('agNumberColumnFilter');
  });

  it('el tooltip sólo muestra texto o números, ya formateados', () => {
    const con = (parcial: Partial<ITooltipParams>) => parcial as ITooltipParams;
    expect(textoDeTooltip(con({ value: 'ANA', valueFormatted: '' as unknown as string }))).toBe('');
    expect(textoDeTooltip(con({ value: '2026-10-09', valueFormatted: '09/10/2026' }))).toBe(
      '09/10/2026',
    );
    expect(textoDeTooltip(con({ value: 7 }))).toBe('7');
    expect(textoDeTooltip(con({ value: { raro: true } }))).toBeUndefined();
  });

  it('la clave de una columna es su colId o su field', () => {
    expect(claveDeColumna({ colId: 'estado', field: 'x' })).toBe('estado');
    expect(claveDeColumna({ field: 'cedula' })).toBe('cedula');
    expect(claveDeColumna({ headerName: 'Acción' })).toBeUndefined();
  });
});
