import { Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import type { ColDef } from 'ag-grid-community';
import { Tabla } from './tabla';

interface Persona {
  id: string;
  nombre: string;
}

@Component({
  imports: [Tabla],
  template: `
    <app-tabla
      etiqueta="Personas"
      [columnas]="columnas"
      [filas]="filas"
      [cargando]="false"
      [idFila]="idFila"
    />
  `,
})
class Anfitrion {
  columnas: ColDef<Persona>[] = [{ field: 'nombre', headerName: 'Nombre' }];
  filas: Persona[] = [
    { id: '1', nombre: 'ANA' },
    { id: '2', nombre: 'BETO' },
    { id: '3', nombre: 'CARLA' },
  ];
  idFila = (fila: Persona): string => fila.id;
}

async function crear() {
  TestBed.configureTestingModule({ imports: [Anfitrion] });
  const fixture = TestBed.createComponent(Anfitrion);
  await fixture.whenStable();
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

describe('Tabla (la grilla base de toda la app)', () => {
  it('se anuncia con su nombre y ofrece un buscador con etiqueta', async () => {
    const { raiz } = await crear();

    const region = raiz.querySelector('section[role="region"]');
    expect(region?.getAttribute('aria-label')).toBe('Personas');
    expect(raiz.querySelector('mat-label')?.textContent).toContain('Buscar en Personas');
    expect(raiz.querySelector('input[type="search"]')).not.toBeNull();
  });

  it('cuenta los registros y lo anuncia con un aviso cortés', async () => {
    const { raiz } = await crear();

    const conteo = raiz.querySelector('.conteo');
    expect(conteo?.textContent?.trim()).toBe('3 registros');
    expect(conteo?.getAttribute('role')).toBe('status');
    expect(conteo?.getAttribute('aria-live')).toBe('polite');
  });

  it('usa AG Grid con el tema de serie y los textos en español', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('ag-grid-angular')).not.toBeNull();
  });
});
