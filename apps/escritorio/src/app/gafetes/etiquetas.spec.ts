import {
  etiquetaDeEstadoGafete,
  etiquetaDeTipoGafete,
  TIPOS_DE_GAFETE,
  textoAsignado,
} from './etiquetas';

describe('etiquetas de gafetes', () => {
  it('ofrece los cuatro tipos con nombre legible', () => {
    expect(TIPOS_DE_GAFETE.map((t) => t.tipo)).toEqual([
      'CONTRATISTA',
      'VISITA',
      'PROVEEDOR',
      'PROVISIONAL_KOF',
    ]);
    expect(etiquetaDeTipoGafete('PROVISIONAL_KOF')).toBe('KOF provisional');
  });

  it('un gafete prestado se ve "Asignado" aunque su estado sea disponible', () => {
    expect(etiquetaDeEstadoGafete({ estado: 'DISPONIBLE', prestado: true })).toBe('Asignado');
    expect(etiquetaDeEstadoGafete({ estado: 'DISPONIBLE', prestado: false })).toBe('Disponible');
    expect(etiquetaDeEstadoGafete({ estado: 'PERDIDO', prestado: false })).toBe('Perdido');
    expect(etiquetaDeEstadoGafete({ estado: 'DE_BAJA', prestado: false })).toBe('De baja');
  });

  it('a quién está asignado: nombre y cédula, sólo la cédula, o nada', () => {
    expect(textoAsignado({ asignado_nombre: 'ANA SOLANO', asignado_identidad: '111111111' })).toBe(
      'ANA SOLANO (111111111)',
    );
    expect(textoAsignado({ asignado_nombre: null, asignado_identidad: '333333333' })).toBe(
      '333333333',
    );
    expect(textoAsignado({ asignado_nombre: null, asignado_identidad: null })).toBe('');
  });
});
