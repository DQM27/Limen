import { describe, expect, it } from 'vitest';
import { fechaHora } from './fecha';

describe('fechaHora', () => {
  it('usa 24 horas: la tarde no lleva p. m.', () => {
    // 17:26:20 en la zona de quien corre la prueba, sin importar cuál sea.
    const tarde = new Date(2026, 9, 14, 17, 26, 20).toISOString();
    const texto = fechaHora(tarde);
    expect(texto).toBe('14/10/2026 17:26:20');
    expect(texto).not.toMatch(/[ap]\.?\s?m/i);
  });

  it('la medianoche es 00, no 12 a. m.', () => {
    expect(fechaHora(new Date(2026, 9, 14, 0, 5, 9).toISOString())).toBe('14/10/2026 00:05:09');
  });

  it('lo vacío o ilegible no muestra nada', () => {
    expect(fechaHora(null)).toBe('');
    expect(fechaHora(undefined)).toBe('');
    expect(fechaHora('no es una fecha')).toBe('');
  });
});
