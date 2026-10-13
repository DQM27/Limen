import { TestBed } from '@angular/core/testing';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { PantallaEntrada } from './pantalla-entrada';
import { SesionServicio } from '../nucleo/sesion';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

const ana = { id: 'abc', cedula: '111111111', nombre: 'ANA MORA', debe_cambiar_clave: false };

async function crear(respuestas: Record<string, unknown> = {}) {
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(invoke).mockImplementation((comando: string) => {
    const respuesta = respuestas[comando];
    return respuesta instanceof Error || (respuesta as { codigo?: string } | undefined)?.codigo
      ? Promise.reject(respuesta)
      : Promise.resolve(respuesta);
  });
  const fixture = TestBed.createComponent(PantallaEntrada);
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  return { fixture, raiz, sesion: TestBed.inject(SesionServicio) };
}

function escribir(raiz: HTMLElement, id: string, texto: string): void {
  const campo = raiz.querySelector<HTMLInputElement>(`#${id}`);
  if (!campo) {
    throw new Error(`no hay campo ${id}`);
  }
  campo.value = texto;
  campo.dispatchEvent(new Event('input'));
}

async function enviar(fixture: { whenStable(): Promise<unknown> }, raiz: HTMLElement) {
  raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
  await fixture.whenStable();
}

describe('pantalla de entrada', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(isTauri).mockReset();
    localStorage.clear();
  });

  it('pide cédula y clave, con la clave oculta', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector<HTMLInputElement>('#cedula')).not.toBeNull();
    expect(raiz.querySelector<HTMLInputElement>('#clave')?.type).toBe('password');
    expect(raiz.textContent).toContain('Ingresar');
  });

  it('entra con la cédula y la clave que se escribieron', async () => {
    const { fixture, raiz, sesion } = await crear({ iniciar_sesion: ana });

    escribir(raiz, 'cedula', ' 111111111 ');
    escribir(raiz, 'clave', 'secreta');
    await enviar(fixture, raiz);

    expect(invoke).toHaveBeenCalledWith('iniciar_sesion', {
      cedula: '111111111',
      clave: 'secreta',
    });
    expect(sesion.usuario()).toEqual(ana);
  });

  it('muestra el mensaje del núcleo tal cual y borra la clave si la entrada falla', async () => {
    const { fixture, raiz, sesion } = await crear({
      iniciar_sesion: {
        tipo: 'negocio',
        codigo: 'credenciales_invalidas',
        mensaje: 'Cédula o clave incorrecta.',
        campo: null,
      },
    });

    escribir(raiz, 'cedula', '111111111');
    escribir(raiz, 'clave', 'mala');
    await enviar(fixture, raiz);
    fixture.detectChanges();

    expect(raiz.querySelector('.error.general')?.textContent).toContain(
      'Cédula o clave incorrecta.',
    );
    expect(raiz.querySelector<HTMLInputElement>('#clave')?.value).toBe('');
    expect(sesion.usuario()).toBeNull();
  });

  it('recuerda la cédula, nunca la clave, si se marca "Recordar usuario"', async () => {
    const { fixture, raiz } = await crear({ iniciar_sesion: ana });

    escribir(raiz, 'cedula', '111111111');
    escribir(raiz, 'clave', 'secreta');
    await enviar(fixture, raiz);

    expect(localStorage.getItem('limen.usuario-recordado')).toBeNull();
  });

  it('con la cédula recordada la precarga y deja marcada la casilla', async () => {
    localStorage.setItem('limen.usuario-recordado', '111111111');
    const { raiz } = await crear();

    expect(raiz.querySelector<HTMLInputElement>('#cedula')?.value).toBe('111111111');
    expect(raiz.querySelector<HTMLInputElement>('#clave')?.value).toBe('');
    expect(raiz.querySelector('mat-checkbox')?.classList).toContain('mat-mdc-checkbox-checked');
  });

  it('con una clave temporal sólo ofrece cambiarla y no deja seguir con claves distintas', async () => {
    const { fixture, raiz, sesion } = await crear({ cambiar_clave: undefined });
    sesion['_usuario'].set({ ...ana, debe_cambiar_clave: true });
    fixture.detectChanges();

    expect(raiz.querySelector('#cedula')).toBeNull();
    expect(raiz.textContent).toContain('Cambiar clave');

    escribir(raiz, 'clave-actual', 'temporal');
    escribir(raiz, 'clave-nueva', 'nueva-clave-1');
    escribir(raiz, 'confirmacion', 'otra-distinta');
    await enviar(fixture, raiz);
    fixture.detectChanges();

    expect(invoke).not.toHaveBeenCalledWith('cambiar_clave', expect.anything());
    expect(raiz.querySelector('#error-confirmacion')?.textContent).toContain('no coinciden');
  });

  it('cambia la clave y pone el error del núcleo junto a su campo', async () => {
    const { fixture, raiz, sesion } = await crear({
      cambiar_clave: {
        tipo: 'negocio',
        codigo: 'clave_corta',
        mensaje: 'La clave es muy corta.',
        campo: 'clave',
      },
    });
    sesion['_usuario'].set({ ...ana, debe_cambiar_clave: true });
    fixture.detectChanges();

    escribir(raiz, 'clave-actual', 'temporal');
    escribir(raiz, 'clave-nueva', 'x');
    escribir(raiz, 'confirmacion', 'x');
    await enviar(fixture, raiz);
    fixture.detectChanges();

    expect(invoke).toHaveBeenCalledWith('cambiar_clave', { claveActual: 'temporal', clave: 'x' });
    expect(raiz.querySelector('#error-clave-nueva')?.textContent).toContain('muy corta');
  });

  it('"Cancelar" cierra la app desde la entrada', async () => {
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button[type=button]:not(.ojo)')?.click();
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('salir', undefined);
  });
});
