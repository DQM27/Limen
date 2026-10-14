import { beforeEach, describe, expect, it } from 'vitest';
import { claveDeColumna, operacionDeFiltro } from './columnas';
import { claveDeDiseno, guardarDiseno, leerDiseno } from './diseno-guardado';

describe('operacionDeFiltro', () => {
  it('traduce las opciones del filtro de AG Grid a las del núcleo (regla N2)', () => {
    expect(operacionDeFiltro('contains')).toBe('CONTIENE');
    expect(operacionDeFiltro('notContains')).toBe('NO_CONTIENE');
    expect(operacionDeFiltro('equals')).toBe('IGUAL');
    expect(operacionDeFiltro('notEqual')).toBe('DISTINTO');
    expect(operacionDeFiltro('startsWith')).toBe('EMPIEZA_CON');
    expect(operacionDeFiltro('endsWith')).toBe('TERMINA_CON');
  });

  it('una opción desconocida o ausente es «contiene»', () => {
    expect(operacionDeFiltro('rara')).toBe('CONTIENE');
    expect(operacionDeFiltro(null)).toBe('CONTIENE');
    expect(operacionDeFiltro(undefined)).toBe('CONTIENE');
  });
});

describe('claveDeColumna', () => {
  it('es el colId o, si no lo tiene, el field', () => {
    expect(claveDeColumna({ colId: 'acciones', field: 'x' })).toBe('acciones');
    expect(claveDeColumna({ field: 'nombre' })).toBe('nombre');
    expect(claveDeColumna({ headerName: 'sin clave' })).toBeUndefined();
  });
});

describe('diseño guardado de una tabla', () => {
  beforeEach(() => localStorage.clear());

  it('lo que se guarda vuelve igual, por tabla', () => {
    guardarDiseno('ingresos', {
      ocultas: ['unidad'],
      columnas: [{ colId: 'nombre', width: 240 }],
      filtrosVisibles: false,
    });

    expect(leerDiseno('ingresos')).toEqual({
      ocultas: ['unidad'],
      columnas: [{ colId: 'nombre', width: 240 }],
      filtrosVisibles: false,
    });
    expect(leerDiseno('usuarios'), 'cada tabla tiene el suyo').toBeNull();
  });

  it('sin identificador no se guarda ni se lee nada', () => {
    guardarDiseno(undefined, { ocultas: [], columnas: [], filtrosVisibles: true });
    expect(localStorage.length).toBe(0);
    expect(leerDiseno(undefined)).toBeNull();
  });

  it('lo guardado que no sirve se ignora y la tabla arranca como siempre', () => {
    localStorage.setItem(claveDeDiseno('mala'), 'esto no es json');
    expect(leerDiseno('mala')).toBeNull();

    localStorage.setItem(
      claveDeDiseno('rara'),
      JSON.stringify({ ocultas: [1, 'a'], columnas: 'no', filtrosVisibles: 'sí' }),
    );
    expect(leerDiseno('rara')).toEqual({ ocultas: ['a'], columnas: [], filtrosVisibles: true });
  });
});
