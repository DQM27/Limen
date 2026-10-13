import { TestBed } from '@angular/core/testing';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { PantallaVinculacion } from './pantalla-vinculacion';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

async function crear(respuestas: Record<string, unknown> = {}, desarrollo = false) {
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(invoke).mockImplementation((comando: string) => {
    const respuesta = respuestas[comando];
    return (respuesta as { codigo?: string } | undefined)?.codigo
      ? Promise.reject(respuesta)
      : Promise.resolve(respuesta);
  });
  const fixture = TestBed.createComponent(PantallaVinculacion);
  fixture.componentRef.setInput('desarrollo', desarrollo);
  await fixture.whenStable();
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

function escribir(raiz: HTMLElement, texto: string): void {
  const campo = raiz.querySelector<HTMLInputElement>('#codigo-vinculacion');
  if (!campo) {
    throw new Error('no hay campo de código');
  }
  campo.value = texto;
  campo.dispatchEvent(new Event('input'));
}

async function enviar(fixture: { whenStable(): Promise<unknown> }, raiz: HTMLElement) {
  raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
  await fixture.whenStable();
}

describe('pantalla de vinculación', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(isTauri).mockReset();
  });

  it('pide el código del administrador', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('#codigo-vinculacion')).not.toBeNull();
    expect(raiz.textContent).toContain('no está vinculado');
  });

  it('manda el código tal como se escribió: el núcleo lo interpreta', async () => {
    const { fixture, raiz } = await crear({ vincular_equipo_con_codigo: null });

    escribir(raiz, 'ab3de-fgh45');
    await enviar(fixture, raiz);

    expect(invoke).toHaveBeenCalledWith('vincular_equipo_con_codigo', {
      codigo: 'ab3de-fgh45',
    });
  });

  it('al vincular, reinicia la app para abrir la base con la identidad nueva', async () => {
    const { fixture, raiz } = await crear({ vincular_equipo_con_codigo: null, reiniciar: true });

    escribir(raiz, 'AB3DEFGH45');
    await enviar(fixture, raiz);

    expect(invoke).toHaveBeenCalledWith('reiniciar', undefined);
    expect(raiz.textContent).toContain('Reiniciando');
  });

  it('en desarrollo no se reinicia sola: pide abrirla de nuevo', async () => {
    const { fixture, raiz } = await crear({ vincular_equipo_con_codigo: null, reiniciar: false });

    escribir(raiz, 'AB3DEFGH45');
    await enviar(fixture, raiz);

    expect(raiz.textContent).toContain('Cierra la aplicación y ábrela de nuevo');
  });

  it('el error del código se muestra junto al campo y no reinicia', async () => {
    const { fixture, raiz } = await crear({
      vincular_equipo_con_codigo: {
        tipo: 'negocio',
        codigo: 'codigo_vencido',
        mensaje: 'El código venció: pida uno nuevo al administrador',
        campo: 'codigo',
      },
    });

    escribir(raiz, 'AB3DEFGH45');
    await enviar(fixture, raiz);

    expect(raiz.querySelector('#error-codigo')?.textContent).toContain('venció');
    expect(invoke).not.toHaveBeenCalledWith('reiniciar', undefined);
  });

  it('un error sin campo (sin red) se muestra como mensaje general', async () => {
    const { fixture, raiz } = await crear({
      vincular_equipo_con_codigo: {
        tipo: 'tecnico',
        codigo: 'sin_conexion',
        mensaje: 'No hay conexión con el panel. Revise la red e intente de nuevo',
        campo: null,
      },
    });

    escribir(raiz, 'AB3DEFGH45');
    await enviar(fixture, raiz);

    expect(raiz.querySelector('.error.general')?.textContent).toContain('No hay conexión');
    expect(raiz.querySelector('#error-codigo')).toBeNull();
  });

  it('seguir sin vincular sólo se ofrece al desarrollar', async () => {
    const produccion = await crear({}, false);
    expect(produccion.raiz.textContent).not.toContain('Seguir sin vincular');
    TestBed.resetTestingModule();

    const desarrollo = await crear({}, true);
    expect(desarrollo.raiz.textContent).toContain('Seguir sin vincular');
  });
});
