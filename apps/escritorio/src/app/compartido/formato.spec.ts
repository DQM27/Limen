import { fechaCorta, fechaYHora } from './formato';

describe('formatos para mostrar', () => {
  it('fechaCorta pone el día primero y tolera lo vacío', () => {
    expect(fechaCorta('2026-10-09')).toBe('09/10/2026');
    expect(fechaCorta('2099-01-01')).toBe('01/01/2099');
    expect(fechaCorta('2026-10-09T14:00:00Z')).toBe('09/10/2026');
    expect(fechaCorta(null)).toBe('—');
    expect(fechaCorta(undefined)).toBe('—');
    expect(fechaCorta('')).toBe('—');
  });

  it('fechaYHora muestra la hora de Costa Rica (UTC-6) y no la del equipo', () => {
    expect(fechaYHora('2026-10-09T14:30:00Z')).toBe('09/10/2026 08:30');
    // Pasada la medianoche en UTC todavía es el día anterior en Costa Rica.
    expect(fechaYHora('2026-10-10T03:05:00Z')).toBe('09/10/2026 21:05');
    expect(fechaYHora(null)).toBe('—');
  });

  it('un texto que no es una fecha se muestra tal cual en vez de fallar', () => {
    expect(fechaYHora('no es fecha')).toBe('no es fecha');
  });
});
