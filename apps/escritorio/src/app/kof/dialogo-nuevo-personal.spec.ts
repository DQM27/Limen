import { TestBed } from '@angular/core/testing';
import { MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import { DialogoNuevoPersonal } from './dialogo-nuevo-personal';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

type Respuesta = unknown | ((argumentos: Record<string, unknown>) => unknown);

/** Simula el núcleo comando por comando; el nombre al escribir sale en mayúsculas, como en el dominio. */
function responder(respuestas: Record<string, Respuesta> = {}): void {
  const normales: Record<string, Respuesta> = {
    registrar_personal_kof: 'k1',
    nombre_persona_al_escribir: (a: Record<string, unknown>) => String(a['texto']).toUpperCase(),
  };
  vi.mocked(invoke).mockImplementation((comando: string, argumentos?: unknown) => {
    const respuesta = comando in respuestas ? respuestas[comando] : normales[comando];
    const valor =
      typeof respuesta === 'function'
        ? respuesta((argumentos ?? {}) as Record<string, unknown>)
        : respuesta;
    const rechazo = (valor as { codigo?: string } | undefined)?.codigo !== undefined;
    return rechazo ? Promise.reject(valor) : Promise.resolve(valor);
  });
}

function errorDelNucleo(codigo: string, mensaje: string, campo: string | null) {
  return { tipo: 'negocio', codigo, mensaje, campo };
}

async function crear(teclas = new Subject<KeyboardEvent>()) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
    ],
  });
  const fixture = TestBed.createComponent(DialogoNuevoPersonal);
  fixture.detectChanges();
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const componente = fixture.componentInstance;
  const refrescar = async () => {
    await new Promise((resolver) => setTimeout(resolver, 0));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  const campo = (etiqueta: string) =>
    [...raiz.querySelectorAll<HTMLInputElement>('input')].find((i) =>
      i.closest('mat-form-field')?.textContent?.includes(etiqueta),
    );
  const enviar = async () => {
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await refrescar();
  };
  return { fixture, raiz, cerrar, componente, campo, refrescar, enviar };
}

describe('modal de personal KOF nuevo', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
  });

  it('pide el código de empleado y el nombre', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Nuevo personal KOF');
    expect([...raiz.querySelectorAll('mat-label')].map((e) => e.textContent?.trim())).toEqual([
      'Código de empleado',
      'Nombre',
    ]);
  });

  it('registra a la persona y se cierra con su nombre', async () => {
    const { componente, enviar, cerrar } = await crear();
    componente.codigo.setValue('5040017');
    componente.nombre.setValue('ANA MORA');

    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_personal_kof', {
      codigoEmpleado: '5040017',
      nombre: 'ANA MORA',
    });
    expect(cerrar).toHaveBeenCalledWith({ nombre: 'ANA MORA' });
  });

  it('el código sólo admite dígitos y el nombre se ve en mayúsculas al escribirlo', async () => {
    const { campo, componente, refrescar } = await crear();

    const codigo = campo('Código');
    const nombre = campo('Nombre');
    if (!codigo || !nombre) {
      throw new Error('faltan campos');
    }
    codigo.value = '50a40-017';
    codigo.dispatchEvent(new Event('input'));
    nombre.value = 'ana mora';
    nombre.dispatchEvent(new Event('input'));
    await refrescar();

    expect(componente.codigo.value).toBe('5040017');
    expect(componente.nombre.value).toBe('ANA MORA');
  });

  it('cada rechazo del núcleo sale junto a su campo', async () => {
    const casos: [string, string][] = [
      ['codigo_empleado', 'Código de empleado'],
      ['nombre', 'Nombre'],
    ];
    for (const [campo, etiqueta] of casos) {
      TestBed.resetTestingModule();
      vi.mocked(invoke).mockReset();
      responder({ registrar_personal_kof: errorDelNucleo('x', `Falla en ${campo}`, campo) });
      const { enviar, raiz } = await crear();

      await enviar();

      const conError = [...raiz.querySelectorAll('mat-form-field')].find((f) =>
        f.querySelector('mat-error'),
      );
      expect(conError?.querySelector('mat-label')?.textContent?.trim(), campo).toBe(etiqueta);
      expect(conError?.querySelector('mat-error')?.textContent).toContain(`Falla en ${campo}`);
    }
  });

  it('un rechazo sin campo va arriba de los botones', async () => {
    responder({ registrar_personal_kof: errorDelNucleo('sin_sesion', 'Inicia sesión', null) });
    const { enviar, raiz, cerrar } = await crear();

    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Inicia sesión');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('"Cancelar", el botón de cerrar y Escape cierran sin registrar nada', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar } = await crear(teclas);

    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes('Cancelar'))?.click();
    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(3);
    expect(invoke).not.toHaveBeenCalledWith('registrar_personal_kof', expect.anything());
  });
});
