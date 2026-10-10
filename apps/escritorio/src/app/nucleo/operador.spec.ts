import { TestBed } from '@angular/core/testing';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { OperadorServicio } from './operador';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

describe('OperadorServicio', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(isTauri).mockReset();
  });

  it('carga el operador de este equipo desde el núcleo', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValue({ id: 'abc', nombre: 'Operador provisional' });
    const servicio = TestBed.inject(OperadorServicio);

    expect(servicio.operador()).toBeNull();
    await servicio.cargar();

    expect(invoke).toHaveBeenCalledWith('operador_actual', undefined);
    expect(servicio.operador()).toEqual({ id: 'abc', nombre: 'Operador provisional' });
  });

  it('fuera de la app no llama al núcleo', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const servicio = TestBed.inject(OperadorServicio);

    await servicio.cargar();

    expect(invoke).not.toHaveBeenCalled();
    expect(servicio.operador()).toBeNull();
  });
});
