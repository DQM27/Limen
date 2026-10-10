import { TestBed } from '@angular/core/testing';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { SesionServicio } from './sesion';
import type { UsuarioActual } from './tipos';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

const ana: UsuarioActual = {
  id: 'abc',
  cedula: '111111111',
  nombre: 'ANA MORA',
  debe_cambiar_clave: true,
};

describe('SesionServicio', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(isTauri).mockReset();
  });

  it('carga quién tiene la sesión desde el núcleo', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValue(ana);
    const sesion = TestBed.inject(SesionServicio);

    expect(sesion.usuario()).toBeNull();
    await sesion.cargar();

    expect(invoke).toHaveBeenCalledWith('usuario_actual', undefined);
    expect(sesion.usuario()).toEqual(ana);
  });

  it('fuera de la app no llama al núcleo', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const sesion = TestBed.inject(SesionServicio);

    await sesion.cargar();

    expect(invoke).not.toHaveBeenCalled();
    expect(sesion.usuario()).toBeNull();
  });

  it('al entrar recuerda al usuario y al salir lo olvida', async () => {
    vi.mocked(invoke).mockResolvedValue(ana);
    const sesion = TestBed.inject(SesionServicio);

    await sesion.iniciar('111111111', 'portería segura');
    expect(invoke).toHaveBeenCalledWith('iniciar_sesion', {
      cedula: '111111111',
      clave: 'portería segura',
    });
    expect(sesion.usuario()).toEqual(ana);

    vi.mocked(invoke).mockResolvedValue(undefined);
    await sesion.cerrar();
    expect(invoke).toHaveBeenCalledWith('cerrar_sesion', undefined);
    expect(sesion.usuario()).toBeNull();
  });

  it('si el núcleo rechaza la entrada, no abre sesión', async () => {
    vi.mocked(invoke).mockRejectedValue({
      tipo: 'negocio',
      codigo: 'credenciales_invalidas',
      mensaje: 'Cédula o clave incorrecta',
      campo: null,
    });
    const sesion = TestBed.inject(SesionServicio);

    await expect(sesion.iniciar('111111111', 'mal')).rejects.toMatchObject({
      codigo: 'credenciales_invalidas',
    });
    expect(sesion.usuario()).toBeNull();
  });

  it('cambiar la clave temporal deja de pedirla', async () => {
    vi.mocked(invoke).mockResolvedValue(ana);
    const sesion = TestBed.inject(SesionServicio);
    await sesion.iniciar('111111111', 'temporal 123');

    vi.mocked(invoke).mockResolvedValue(undefined);
    await sesion.cambiarClave('temporal 123', 'la mía de verdad');

    expect(invoke).toHaveBeenCalledWith('cambiar_clave', {
      claveActual: 'temporal 123',
      clave: 'la mía de verdad',
    });
    expect(sesion.usuario()?.debe_cambiar_clave).toBe(false);
  });
});
