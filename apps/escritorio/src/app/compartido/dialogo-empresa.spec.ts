import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import { DialogoEmpresa } from './dialogo-empresa';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

function errorDelNucleo(codigo: string, mensaje: string) {
  return { tipo: 'negocio', codigo, mensaje, campo: 'nombre' };
}

/** Responde el núcleo por comando; el texto al escribir sale en mayúsculas, como en el dominio. */
function responder(respuestas: Record<string, unknown> = {}): void {
  const normales: Record<string, unknown> = {
    registrar_empresa: 'e9',
    buscar_empresas: [{ id: 'e9', nombre: 'NUEVA S.A.' }],
  };
  vi.mocked(invoke).mockImplementation((comando: string, argumentos?: unknown) => {
    if (comando === 'nombre_empresa_al_escribir') {
      return Promise.resolve(String((argumentos as { texto: string }).texto).toUpperCase());
    }
    const valor = comando in respuestas ? respuestas[comando] : normales[comando];
    const rechazo = (valor as { codigo?: string } | undefined)?.codigo !== undefined;
    return rechazo ? Promise.reject(valor) : Promise.resolve(valor);
  });
}

async function crear(teclas = new Subject<KeyboardEvent>()) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
    ],
  });
  const fixture = TestBed.createComponent(DialogoEmpresa);
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const escribir = async (texto: string) => {
    const campo = raiz.querySelector<HTMLInputElement>('input');
    if (!campo) {
      throw new Error('no hay campo');
    }
    campo.value = texto;
    campo.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  };
  const enviar = async () => {
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  return { fixture, raiz, cerrar, escribir, enviar };
}

describe('modal de empresa nueva', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
  });

  it('pide sólo el nombre y avisa que una empresa creada no se borra', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Nueva empresa');
    expect(raiz.querySelectorAll('input')).toHaveLength(1);
    expect(raiz.textContent).toContain('no se borra');
  });

  it('el nombre se ve en mayúsculas mientras se escribe', async () => {
    const { raiz, escribir } = await crear();

    await escribir('acme s.a.');

    expect(invoke).toHaveBeenCalledWith('nombre_empresa_al_escribir', { texto: 'acme s.a.' });
    expect(raiz.querySelector('input')?.value).toBe('ACME S.A.');
  });

  it('crea la empresa y la devuelve con el nombre como quedó guardado', async () => {
    const { cerrar, escribir, enviar } = await crear();

    await escribir('nueva s.a.');
    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_empresa', { nombre: 'NUEVA S.A.' });
    expect(cerrar).toHaveBeenCalledWith({ id: 'e9', nombre: 'NUEVA S.A.' });
  });

  it('el rechazo del núcleo (por ejemplo, nombre repetido) se ve junto al campo y no se cierra', async () => {
    responder({
      registrar_empresa: errorDelNucleo('nombre_repetido', 'Ya existe una empresa con ese nombre'),
    });
    const { raiz, cerrar, escribir, enviar } = await crear();

    await escribir('acme');
    await enviar();

    expect(raiz.querySelector('mat-error')?.textContent).toContain('Ya existe una empresa');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('una falla que no es del núcleo se muestra arriba de los botones', async () => {
    vi.mocked(invoke).mockImplementation(() => Promise.reject(new Error('boom')));
    const { raiz, cerrar, enviar } = await crear();

    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('No se pudo crear la empresa');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('"Cancelar", el botón de cerrar y Escape cierran sin crear nada', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar } = await crear(teclas);

    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes('Cancelar'))?.click();
    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(3);
    expect(invoke).not.toHaveBeenCalledWith('registrar_empresa', expect.anything());
  });
});

describe('modal de empresa nueva, del catálogo de proveedoras', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('se titula "Nueva empresa proveedora" y usa los comandos de ese catálogo', async () => {
    vi.mocked(invoke).mockImplementation((comando: string, argumentos?: unknown) => {
      if (comando === 'nombre_empresa_al_escribir') {
        return Promise.resolve(String((argumentos as { texto: string }).texto).toUpperCase());
      }
      if (comando === 'registrar_empresa_proveedora') {
        return Promise.resolve('p9');
      }
      if (comando === 'buscar_empresas_proveedoras') {
        return Promise.resolve([{ id: 'p9', nombre: 'LIMPIEZAS DEL SUR' }]);
      }
      return Promise.reject(new Error(`comando inesperado: ${comando}`));
    });
    const cerrar = vi.fn();
    TestBed.configureTestingModule({
      providers: [
        { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => new Subject() } },
        { provide: MAT_DIALOG_DATA, useValue: { catalogo: 'proveedoras' } },
      ],
    });
    const fixture = TestBed.createComponent(DialogoEmpresa);
    await fixture.whenStable();
    const raiz = fixture.nativeElement as HTMLElement;

    expect(raiz.querySelector('h2')?.textContent).toContain('Nueva empresa proveedora');
    const campo = raiz.querySelector<HTMLInputElement>('input');
    if (!campo) {
      throw new Error('no hay campo');
    }
    campo.value = 'limpiezas del sur';
    campo.dispatchEvent(new Event('input'));
    await fixture.whenStable();
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('registrar_empresa_proveedora', {
      nombre: 'LIMPIEZAS DEL SUR',
    });
    expect(invoke).not.toHaveBeenCalledWith('registrar_empresa', expect.anything());
    expect(cerrar).toHaveBeenCalledWith({ id: 'p9', nombre: 'LIMPIEZAS DEL SUR' });
  });
});
