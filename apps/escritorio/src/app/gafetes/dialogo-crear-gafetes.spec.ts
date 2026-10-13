import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import { DialogoCrearGafetes } from './dialogo-crear-gafetes';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

async function crear(teclas = new Subject<KeyboardEvent>()) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
      { provide: MAT_DIALOG_DATA, useValue: { tipo: 'VISITA' } },
    ],
  });
  const fixture = TestBed.createComponent(DialogoCrearGafetes);
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const campos = () => [...raiz.querySelectorAll<HTMLInputElement>('input')];
  const escribir = async (indice: number, texto: string) => {
    const campo = campos()[indice];
    if (!campo) {
      throw new Error(`no hay campo ${indice}`);
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
  return { fixture, raiz, cerrar, escribir, enviar, campos };
}

describe('modal de gafetes nuevos', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(5);
  });

  it('dice de qué tipo son y pide el rango', async () => {
    const { raiz, campos } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Nuevos gafetes');
    expect(raiz.querySelector('.tipo')?.textContent).toContain('Visita');
    expect(campos()).toHaveLength(2);
  });

  it('crea el rango y devuelve cuántos se crearon', async () => {
    const { cerrar, escribir, enviar } = await crear();

    await escribir(0, '1');
    await escribir(1, '5');
    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_gafetes', {
      tipo: 'VISITA',
      desde: 1,
      hasta: 5,
    });
    expect(cerrar).toHaveBeenCalledWith(5);
  });

  it('con "Hasta" vacío crea uno solo', async () => {
    const { escribir, enviar } = await crear();

    await escribir(0, '12');
    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_gafetes', {
      tipo: 'VISITA',
      desde: 12,
      hasta: 12,
    });
  });

  it('sólo deja escribir dígitos', async () => {
    const { campos, escribir } = await crear();

    await escribir(0, '1a2-3');

    expect(campos()[0]?.value).toBe('123');
  });

  it('un campo vacío se manda como 0 y el núcleo dice que el rango no vale', async () => {
    vi.mocked(invoke).mockRejectedValue({
      tipo: 'negocio',
      codigo: 'gafete_rango_invalido',
      mensaje: 'El rango no es válido',
      campo: 'hasta',
    });
    const { raiz, cerrar, enviar } = await crear();

    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_gafetes', {
      tipo: 'VISITA',
      desde: 0,
      hasta: 0,
    });
    expect(raiz.querySelector('mat-error')?.textContent).toContain('El rango no es válido');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('un error que no es del rango (un número repetido) va arriba de los botones', async () => {
    vi.mocked(invoke).mockRejectedValue({
      tipo: 'negocio',
      codigo: 'gafete_repetido',
      mensaje: 'Ya existe un gafete con ese número',
      campo: null,
    });
    const { raiz, escribir, enviar } = await crear();

    await escribir(0, '1');
    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Ya existe un gafete');
    expect(raiz.querySelector('mat-error')).toBeNull();
  });

  it('"Cancelar", el botón de cerrar y Escape cierran sin crear nada', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar } = await crear(teclas);

    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes('Cancelar'))?.click();
    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(3);
    expect(invoke).not.toHaveBeenCalled();
  });
});
