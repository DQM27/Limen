import type { Acceso } from '../nucleo/tipos';
import { etiquetaDeAcceso, etiquetaDeTipo } from './etiquetas';

const acceso = (parcial: Partial<Acceso>): Acceso => ({
  resultado: 'PERMITIDO',
  dias_para_vencer: null,
  motivo: null,
  ...parcial,
});

describe('etiquetas de contratistas', () => {
  it('traducen los códigos del núcleo sin decidir nada', () => {
    expect(etiquetaDeAcceso(acceso({}))).toBe('Vigente');
    expect(
      etiquetaDeAcceso(acceso({ resultado: 'PERMITIDO_CON_ADVERTENCIA', dias_para_vencer: 11 })),
    ).toBe('Por vencer (11 días)');
    expect(
      etiquetaDeAcceso(acceso({ resultado: 'PERMITIDO_CON_ADVERTENCIA', dias_para_vencer: 0 })),
    ).toBe('Vence hoy');
    expect(etiquetaDeAcceso(acceso({ resultado: 'DENEGADO', motivo: 'praind_vencido' }))).toBe(
      'PRAIND vencido',
    );
    expect(etiquetaDeAcceso(acceso({ resultado: 'DENEGADO', motivo: 'sin_acceso' }))).toBe(
      'Sin acceso',
    );
  });

  it('el tipo de ingreso se escribe como lo lee el operador', () => {
    expect(etiquetaDeTipo('PRAIND')).toBe('PRAIND');
    expect(etiquetaDeTipo('IN_HOUSE')).toBe('IN HOUSE');
  });
});
