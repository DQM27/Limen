import type { PersonaAdentro } from '../nucleo/tipos';
import {
  etiquetaDeVia,
  filtrarPorGafete,
  textoGafete,
  textoMedio,
  textoOperador,
} from './etiquetas';

function persona(cambios: Partial<PersonaAdentro> = {}): PersonaAdentro {
  return {
    via: 'CONTRATISTA',
    ingreso_id: 'i1',
    identidad: '111111111',
    nombre: 'ANA SOLANO',
    procedencia: 'ACME S.A.',
    medio: 'A_PIE',
    placa: null,
    gafete: 3,
    sin_gafete: false,
    entrada: '2026-10-09T14:00:00Z',
    entrada_por: { id: 'o1', nombre: 'OPERADOR UNO' },
    demasiado_tiempo: false,
    ...cambios,
  };
}

describe('etiquetas de Dentro', () => {
  it('nombra las cuatro vías', () => {
    expect(etiquetaDeVia('CONTRATISTA')).toBe('Contratista');
    expect(etiquetaDeVia('PROVEEDOR')).toBe('Proveedor');
    expect(etiquetaDeVia('CORREO')).toBe('Correo');
    expect(etiquetaDeVia('KOF')).toBe('KOF');
  });

  it('el medio: a pie, en vehículo con su placa, o nada', () => {
    expect(textoMedio({ medio: 'A_PIE', placa: null })).toBe('A pie');
    expect(textoMedio({ medio: 'VEHICULO', placa: 'ABC-123' })).toBe('Vehículo ABC-123');
    expect(textoMedio({ medio: 'VEHICULO', placa: null })).toBe('Vehículo');
    expect(textoMedio({ medio: null, placa: null })).toBe('');
  });

  it('el gafete: su número, S/G si entró sin gafete, o un guion si no le aplica', () => {
    expect(textoGafete({ gafete: 3, sin_gafete: false })).toBe('3');
    expect(textoGafete({ gafete: null, sin_gafete: true })).toBe('S/G');
    expect(textoGafete({ gafete: null, sin_gafete: false })).toBe('—');
  });

  it('quién dio el ingreso: su nombre o, si no está en el equipo, su ID', () => {
    expect(textoOperador({ id: 'o1', nombre: 'OPERADOR UNO' })).toBe('OPERADOR UNO');
    expect(textoOperador({ id: 'o9', nombre: null })).toBe('o9');
  });

  it('el filtro por gafete separa a quien tiene gafete de quien no', () => {
    const con = persona({ ingreso_id: 'con' });
    const sinGafete = persona({ ingreso_id: 'sg', gafete: null, sin_gafete: true });
    const noAplica = persona({ ingreso_id: 'na', gafete: null });
    const todos = [con, sinGafete, noAplica];

    expect(filtrarPorGafete(todos, 'todos')).toEqual(todos);
    expect(filtrarPorGafete(todos, 'con')).toEqual([con]);
    expect(filtrarPorGafete(todos, 'sin')).toEqual([sinGafete, noAplica]);
  });
});
