import { invoke, isTauri } from '@tauri-apps/api/core';
import { enTauri, esErrorApp, invocar } from './tauri';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

describe('puente a los comandos del núcleo', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(isTauri).mockReset();
  });

  it('invocar pasa el nombre del comando y sus argumentos tal cual', async () => {
    vi.mocked(invoke).mockResolvedValue(['uno']);

    const resultado = await invocar<string[]>('buscar_empresas', { texto: 'acm', limite: 10 });

    expect(resultado).toEqual(['uno']);
    expect(invoke).toHaveBeenCalledWith('buscar_empresas', { texto: 'acm', limite: 10 });
  });

  it('si el comando falla, rechaza con el error del núcleo sin tocarlo', async () => {
    const error = { tipo: 'negocio', codigo: 'cedula_repetida', mensaje: 'Ya existe' };
    vi.mocked(invoke).mockRejectedValue(error);

    await expect(invocar('registrar_contratista')).rejects.toBe(error);
  });

  it('enTauri refleja si corre dentro de la app', () => {
    vi.mocked(isTauri).mockReturnValue(true);
    expect(enTauri()).toBe(true);
    vi.mocked(isTauri).mockReturnValue(false);
    expect(enTauri()).toBe(false);
  });

  it('esErrorApp reconoce la forma exacta del error y nada más', () => {
    expect(esErrorApp({ tipo: 'negocio', codigo: 'x', mensaje: 'y' })).toBe(true);
    expect(esErrorApp({ tipo: 'tecnico', codigo: 'x', mensaje: 'y' })).toBe(true);
    expect(esErrorApp({ tipo: 'otro', codigo: 'x', mensaje: 'y' })).toBe(false);
    expect(esErrorApp({ tipo: 'negocio', codigo: 'x' })).toBe(false);
    expect(esErrorApp(new Error('boom'))).toBe(false);
    expect(esErrorApp(null)).toBe(false);
    expect(esErrorApp('texto')).toBe(false);
  });
});
