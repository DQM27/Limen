import type { TextMatcherParams } from 'ag-grid-community';
import { analizarBusqueda, coincideBusqueda, coincideFiltroDeColumna, plegar } from './pliegue';

const filtro = (
  filterOption: TextMatcherParams['filterOption'],
  value: string,
  filterText: string | null,
): boolean =>
  coincideFiltroDeColumna({ filterOption, value, filterText } as unknown as TextMatcherParams);

describe('búsqueda de la tabla', () => {
  it('pliega tildes y mayúsculas, y la Ñ cuenta como N', () => {
    expect(plegar('Sánchez')).toBe('SANCHEZ');
    expect(plegar('NUÑEZ peña')).toBe('NUNEZ PENA');
  });

  it('el buscador general encuentra por palabras en cualquier orden', () => {
    const fila = 'CARLOS MAURICIO SÁNCHEZ ALDAMA';
    expect(coincideBusqueda(analizarBusqueda('sanchez carlos'), fila)).toBe(true);
    expect(coincideBusqueda(analizarBusqueda('carlos sa'), fila)).toBe(true);
    expect(coincideBusqueda(analizarBusqueda('carlos rojas'), fila)).toBe(false);
  });

  it('un buscador vacío no excluye a nadie', () => {
    expect(coincideBusqueda(analizarBusqueda('   '), 'CUALQUIERA')).toBe(true);
  });

  it('el filtro de columna "contiene" parte por palabras y no distingue tildes', () => {
    expect(filtro('contains', 'Carlos Mauricio Sánchez', 'carlos sa')).toBe(true);
    expect(filtro('contains', 'Carlos Mauricio Sánchez', 'sanchez')).toBe(true);
    expect(filtro('contains', 'Carlos Mauricio Sánchez', 'ana')).toBe(false);
    expect(filtro('notContains', 'Carlos', 'ana')).toBe(true);
  });

  it('las demás opciones comparan una sola frase', () => {
    expect(filtro('equals', 'Ana María', 'ANA MARIA')).toBe(true);
    expect(filtro('notEqual', 'Ana', 'beto')).toBe(true);
    expect(filtro('startsWith', 'Ñandú', 'nan')).toBe(true);
    expect(filtro('endsWith', 'Ñandú', 'ndu')).toBe(true);
  });

  it('sin texto de filtro todo coincide', () => {
    expect(filtro('contains', 'x', null)).toBe(true);
  });
});
