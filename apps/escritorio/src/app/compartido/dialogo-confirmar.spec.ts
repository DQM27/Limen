import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { DialogoConfirmar } from './dialogo-confirmar';

async function crear() {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar } },
      {
        provide: MAT_DIALOG_DATA,
        useValue: { titulo: 'Registrar salida', mensaje: '¿Seguro?', confirmar: 'Sí, registrar' },
      },
    ],
  });
  const fixture = TestBed.createComponent(DialogoConfirmar);
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const boton = (texto: string) =>
    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes(texto));
  return { raiz, cerrar, boton };
}

describe('diálogo de confirmación', () => {
  it('muestra el título, el mensaje y el texto del botón que confirma', async () => {
    const { raiz, boton } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Registrar salida');
    expect(raiz.textContent).toContain('¿Seguro?');
    expect(boton('Sí, registrar')).toBeDefined();
  });

  it('confirmar lo cierra con true y cancelar, sin nada', async () => {
    const { boton, cerrar } = await crear();

    boton('Cancelar')?.click();
    expect(cerrar).toHaveBeenLastCalledWith();

    boton('Sí, registrar')?.click();
    expect(cerrar).toHaveBeenLastCalledWith(true);
  });
});
