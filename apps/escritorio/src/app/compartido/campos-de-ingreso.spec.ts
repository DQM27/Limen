import { Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { FormControl } from '@angular/forms';
import { invoke } from '@tauri-apps/api/core';
import type { Medio } from '../nucleo/tipos';
import { CamposDeIngreso } from './campos-de-ingreso';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

@Component({
  imports: [CamposDeIngreso],
  template: `<app-campos-de-ingreso
    [medio]="medio"
    [placa]="placa"
    [gafete]="gafete"
    etiquetaGafete="Gafete de visita"
  />`,
})
class Anfitrion {
  medio = new FormControl<Medio>('A_PIE', { nonNullable: true });
  placa = new FormControl('', { nonNullable: true });
  gafete = new FormControl('', { nonNullable: true });
}

async function crear() {
  const fixture = TestBed.createComponent(Anfitrion);
  fixture.detectChanges();
  await fixture.whenStable();
  const refrescar = async () => {
    await new Promise((resolver) => setTimeout(resolver, 0));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  return {
    fixture,
    anfitrion: fixture.componentInstance,
    raiz: fixture.nativeElement as HTMLElement,
    refrescar,
  };
}

describe('campos de ingreso (medio, placa y gafete)', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation((_comando: string, argumentos?: unknown) =>
      Promise.resolve(String((argumentos as { texto: string }).texto).toUpperCase()),
    );
  });

  it('empieza a pie: pide el gafete y no la placa', async () => {
    const { raiz } = await crear();

    const etiquetas = [...raiz.querySelectorAll('mat-label')].map((e) => e.textContent?.trim());
    expect(etiquetas).toEqual(['Gafete de visita']);
    expect(
      [...raiz.querySelectorAll('mat-button-toggle')].map((t) => t.textContent?.trim()),
    ).toEqual(['A pie', 'En vehículo']);
  });

  it('en vehículo pide también la placa, sin importar quién cambie el medio', async () => {
    const { anfitrion, raiz, refrescar } = await crear();

    anfitrion.medio.setValue('VEHICULO');
    await refrescar();

    const etiquetas = [...raiz.querySelectorAll('mat-label')].map((e) => e.textContent?.trim());
    expect(etiquetas).toEqual(['Placa', 'Gafete de visita']);

    anfitrion.medio.setValue('A_PIE');
    await refrescar();
    expect([...raiz.querySelectorAll('mat-label')]).toHaveLength(1);
  });

  it('elegir "En vehículo" con el ratón cambia el control', async () => {
    const { anfitrion, raiz, refrescar } = await crear();

    raiz.querySelectorAll<HTMLButtonElement>('mat-button-toggle button')[1]?.click();
    await refrescar();

    expect(anfitrion.medio.value).toBe('VEHICULO');
    expect(raiz.textContent).toContain('Placa');
  });

  it('la placa se ve en mayúsculas mientras se escribe, como lo decide el dominio', async () => {
    const { anfitrion, raiz, refrescar } = await crear();
    anfitrion.medio.setValue('VEHICULO');
    await refrescar();
    const campo = [...raiz.querySelectorAll<HTMLInputElement>('input')].find((i) =>
      i.closest('mat-form-field')?.textContent?.includes('Placa'),
    );
    if (!campo) {
      throw new Error('no hay campo de placa');
    }

    campo.value = 'abc-123';
    campo.dispatchEvent(new Event('input'));
    await refrescar();

    expect(invoke).toHaveBeenCalledWith('placa_al_escribir', { texto: 'abc-123' });
    expect(anfitrion.placa.value).toBe('ABC-123');
  });

  it('el gafete sólo admite dígitos', async () => {
    const { anfitrion, raiz, refrescar } = await crear();
    const campo = raiz.querySelector<HTMLInputElement>('input[inputmode="numeric"]');
    if (!campo) {
      throw new Error('no hay campo de gafete');
    }

    campo.value = '1a2-3';
    campo.dispatchEvent(new Event('input'));
    await refrescar();

    expect(anfitrion.gafete.value).toBe('123');
    expect(campo.value).toBe('123');
  });

  it('los errores del núcleo puestos en los controles se ven junto a cada campo', async () => {
    const { anfitrion, raiz, refrescar } = await crear();
    anfitrion.medio.setValue('VEHICULO');
    await refrescar();

    anfitrion.placa.setErrors({ nucleo: 'La placa es obligatoria' });
    anfitrion.placa.markAsTouched();
    anfitrion.gafete.setErrors({ nucleo: 'El gafete no existe' });
    anfitrion.gafete.markAsTouched();
    await refrescar();

    expect([...raiz.querySelectorAll('mat-error')].map((e) => e.textContent?.trim())).toEqual([
      'La placa es obligatoria',
      'El gafete no existe',
    ]);
  });
});
