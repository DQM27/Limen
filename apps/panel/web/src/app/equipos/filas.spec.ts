import { describe, expect, it } from 'vitest';
import type { Dispositivo, EquipoConectado } from '../panel';
import { unirEquipos } from './filas';

const dispositivo = (
  id: string,
  nombre: string,
  extra: Partial<Dispositivo> = {},
): Dispositivo => ({
  id,
  nombre,
  unidad: 'BRISAS',
  tipo: 'pc',
  creado: '2026-10-13T20:00:00Z',
  vinculado: '2026-10-13T20:05:00Z',
  retirado: null,
  estado: 'vinculado',
  ...extra,
});

const conectado = (equipo: string, estado: EquipoConectado['estado']): EquipoConectado => ({
  equipo,
  unidad: 'BRISAS',
  iniciada: '2026-10-14T14:00:00Z',
  visto: '2026-10-14T14:30:00Z',
  estado,
});

describe('unirEquipos', () => {
  it('une cada equipo registrado con su conexión por el ID', () => {
    const filas = unirEquipos([dispositivo('a1', 'A1')], [conectado('a1', 'en_linea')]);
    expect(filas).toHaveLength(1);
    expect(filas[0]).toMatchObject({
      nombre: 'A1',
      registro: 'vinculado',
      conexion: 'en_linea',
      visto: '2026-10-14T14:30:00Z',
    });
  });

  it('un equipo registrado que nunca se conectó queda sin conexión', () => {
    const [fila] = unirEquipos([dispositivo('a1', 'A1', { estado: 'esperando_codigo' })], []);
    expect(fila?.conexion).toBeNull();
    expect(fila?.visto).toBeNull();
    expect(fila?.registro).toBe('esperando_codigo');
  });

  it('agrega al final los que se conectaron sin estar registrados', () => {
    const filas = unirEquipos([dispositivo('a1', 'A1')], [conectado('9df9696d-0000', 'caido')]);
    expect(filas[0]?.registro).toBe('vinculado');
    expect(filas[1]?.registro).toBe('sin_registrar');
    expect(filas[1]).toMatchObject({
      nombre: '9df9696d',
      conexion: 'caido',
      iniciada: '2026-10-14T14:00:00Z',
      creado: null,
    });
  });

  it('ordena por unidad y luego por nombre', () => {
    const filas = unirEquipos(
      [
        dispositivo('c', 'B2', { unidad: 'CARTAGO' }),
        dispositivo('b', 'B2'),
        dispositivo('a', 'A1'),
      ],
      [],
    );
    expect(filas.map((f) => `${f.unidad}/${f.nombre}`)).toEqual([
      'BRISAS/A1',
      'BRISAS/B2',
      'CARTAGO/B2',
    ]);
  });
});
