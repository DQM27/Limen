import { TestBed } from '@angular/core/testing';
import { invoke } from '@tauri-apps/api/core';
import { DentroPagina } from './dentro-pagina';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const persona = (id: string, nombre: string, via: string) => ({
  via,
  ingreso_id: id,
  identidad: id,
  nombre,
  procedencia: 'ACME',
  medio: null,
  placa: null,
  gafete: null,
  sin_gafete: false,
  entrada: '2026-10-09T14:00:00Z',
  entrada_por: { id: 'o1', nombre: 'OPERADOR UNO' },
  demasiado_tiempo: false,
});

describe('pantalla Dentro', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('muestra a quienes están adentro por las cuatro vías, sin filtrar', async () => {
    vi.mocked(invoke).mockResolvedValue([
      persona('1', 'ANA', 'CONTRATISTA'),
      persona('2', 'BETO', 'PROVEEDOR'),
      persona('3', 'CARLA', 'KOF'),
    ]);
    const fixture = TestBed.createComponent(DentroPagina);
    fixture.detectChanges();
    await new Promise((resolver) => setTimeout(resolver, 0));
    await fixture.whenStable();
    fixture.detectChanges();
    const raiz = fixture.nativeElement as HTMLElement;

    expect(raiz.querySelector('h1')?.textContent).toBe('Dentro');
    await vi.waitFor(() =>
      expect(raiz.querySelector('.conteo')?.textContent?.trim()).toBe('3 registros'),
    );
  });
});
