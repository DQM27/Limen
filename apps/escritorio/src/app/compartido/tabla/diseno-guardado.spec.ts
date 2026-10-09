import { claveDeDiseno, guardarDiseno, leerDiseno } from './diseno-guardado';

describe('diseño guardado de la tabla', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('lo que se guarda se lee igual, por tabla', () => {
    guardarDiseno('contratistas', {
      ocultas: ['tipo_ingreso'],
      columnas: [{ colId: 'cedula', width: 150 }],
      filtrosVisibles: false,
    });

    expect(leerDiseno('contratistas')).toEqual({
      ocultas: ['tipo_ingreso'],
      columnas: [{ colId: 'cedula', width: 150 }],
      filtrosVisibles: false,
    });
    expect(leerDiseno('historial'), 'otra tabla no se ve afectada').toBeNull();
  });

  it('sin id no guarda ni lee nada', () => {
    guardarDiseno(undefined, { ocultas: [], columnas: [], filtrosVisibles: true });
    expect(leerDiseno(undefined)).toBeNull();
    expect(localStorage.length).toBe(0);
  });

  it('lo guardado que no sirve se ignora en vez de romper la tabla', () => {
    localStorage.setItem(claveDeDiseno('a'), 'esto no es json');
    expect(leerDiseno('a')).toBeNull();
    localStorage.setItem(claveDeDiseno('b'), '42');
    expect(leerDiseno('b')).toBeNull();
    localStorage.setItem(
      claveDeDiseno('c'),
      JSON.stringify({ ocultas: 'no', filtrosVisibles: 'sí' }),
    );
    expect(leerDiseno('c')).toEqual({ ocultas: [], columnas: [], filtrosVisibles: true });
  });

  it('si el almacenamiento falla, no lanza', () => {
    const original = Storage.prototype.setItem;
    Storage.prototype.setItem = () => {
      throw new Error('cuota llena');
    };
    try {
      expect(() =>
        guardarDiseno('x', { ocultas: [], columnas: [], filtrosVisibles: true }),
      ).not.toThrow();
    } finally {
      Storage.prototype.setItem = original;
    }
  });
});
