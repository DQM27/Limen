import type { ICellRendererParams } from 'ag-grid-community';
import type { Segmento } from '../../nucleo/tipos';
import { campoResaltable, claveDeTexto, pintarCelda, resaltable } from './resaltado';

function celda(
  valor: unknown,
  segmentos?: readonly Segmento[],
  campo: string | null = 'NOMBRE',
): HTMLElement {
  const parametros = {
    value: valor,
    campo: campo ?? undefined,
    context: { segmentosDe: () => segmentos },
  } as unknown as ICellRendererParams;
  return pintarCelda(parametros);
}

describe('resaltado de celdas (regla N1)', () => {
  it('pinta en <mark> lo resaltado y el resto como texto, sin perder nada', () => {
    const elemento = celda('José Peña', [
      { texto: 'Jos', resaltado: true },
      { texto: 'é ', resaltado: false },
      { texto: 'Pe', resaltado: true },
      { texto: 'ña', resaltado: false },
    ]);

    expect([...elemento.querySelectorAll('mark')].map((m) => m.textContent)).toEqual(['Jos', 'Pe']);
    expect(elemento.textContent).toBe('José Peña');
  });

  it('sin segmentos muestra el texto tal cual', () => {
    const elemento = celda('José Peña', undefined);

    expect(elemento.querySelector('mark')).toBeNull();
    expect(elemento.textContent).toBe('José Peña');
  });

  it('un valor vacío se muestra vacío, no como "undefined"', () => {
    expect(celda(undefined, undefined).textContent).toBe('');
    expect(celda(null, undefined).textContent).toBe('');
  });

  it('nunca interpreta el texto como HTML', () => {
    const peligroso = '<img src=x onerror="alert(1)">';
    const elemento = celda(peligroso, [
      { texto: '<img src=x ', resaltado: true },
      { texto: 'onerror="alert(1)">', resaltado: false },
    ]);

    expect(elemento.querySelector('img')).toBeNull();
    expect(elemento.textContent).toBe(peligroso);
  });

  it('sin campo no pide segmentos (una celda que no es resaltable)', () => {
    const elemento = celda('José', [{ texto: 'José', resaltado: true }], null);

    expect(elemento.querySelector('mark')).toBeNull();
  });

  it('una columna resaltable recuerda su campo y deja el resto como estaba', () => {
    const columna = resaltable({ field: 'nombre', headerName: 'Nombre' }, 'NOMBRE');

    expect(campoResaltable(columna)).toBe('NOMBRE');
    expect(columna.field).toBe('nombre');
    expect(columna.headerName).toBe('Nombre');
    expect(campoResaltable({ field: 'otra' })).toBeUndefined();
  });

  it('la clave distingue el campo del texto', () => {
    expect(claveDeTexto('NOMBRE', 'ANA')).not.toBe(claveDeTexto('EMPRESA', 'ANA'));
  });
});
