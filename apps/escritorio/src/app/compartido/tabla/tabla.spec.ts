import { Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import type { ColDef } from 'ag-grid-community';
import { claveDeDiseno } from './diseno-guardado';
import { resaltable, type Resaltador } from './resaltado';
import { Tabla } from './tabla';

interface Persona {
  id: string;
  nombre: string;
  empresa: string;
}

@Component({
  imports: [Tabla],
  template: `
    <app-tabla
      id="personas"
      etiqueta="Personas"
      marcador="Nombre, empresa…"
      [columnas]="columnas"
      [filas]="filas"
      [idFila]="idFila"
    >
      <button controles type="button" class="propio">Nuevo</button>
      <span acciones class="extra">Rango</span>
    </app-tabla>
  `,
})
class Anfitrion {
  columnas: ColDef<Persona>[] = [
    { field: 'nombre', headerName: 'Nombre' },
    { field: 'empresa', headerName: 'Empresa' },
  ];
  filas: Persona[] = [
    { id: '1', nombre: 'ANA', empresa: 'ACME' },
    { id: '2', nombre: 'BETO', empresa: 'ACME' },
    { id: '3', nombre: 'CARLA', empresa: 'ZETA' },
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
  beforeEach(() => {
    localStorage.clear();
  });

  it('se anuncia con su nombre y ofrece un buscador con etiqueta y marcador', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('section[role="region"]')?.getAttribute('aria-label')).toBe(
      'Personas',
    );
    const buscador = raiz.querySelector<HTMLInputElement>('input[type="search"]');
    expect(buscador?.getAttribute('aria-label')).toBe('Buscar en Personas');
    expect(buscador?.placeholder).toBe('Nombre, empresa…');
  });

  it('cuenta los registros y lo anuncia con un aviso cortés', async () => {
    const { raiz } = await crear();

    const conteo = raiz.querySelector('.conteo');
    expect(conteo?.textContent?.trim()).toBe('3 registros');
    expect(conteo?.getAttribute('role')).toBe('status');
    expect(conteo?.getAttribute('aria-live')).toBe('polite');
  });

  it('deja que cada pantalla ponga sus controles a la izquierda y sus acciones a la derecha', async () => {
    const { raiz } = await crear();

    const izquierda = raiz.querySelector('.herramientas .grupo:first-child');
    const derecha = raiz.querySelector('.herramientas .grupo:last-child');
    expect(izquierda?.querySelector('button.propio')?.textContent).toBe('Nuevo');
    expect(derecha?.querySelector('span.extra')?.textContent).toBe('Rango');
  });

  it('las herramientas de la grilla van en un solo grupo, todas con nombre accesible', async () => {
    const { raiz } = await crear();

    const grupo = raiz.querySelector('.segmentado');
    expect(grupo?.getAttribute('role')).toBe('group');
    const nombres = [...(grupo?.querySelectorAll('button') ?? [])].map((b) =>
      b.getAttribute('aria-label'),
    );
    expect(nombres).toEqual([
      'Ocultar filtros',
      'Ajustar anchos al contenido',
      'Restablecer anchos',
      'Columnas visibles',
    ]);
  });

  it('el interruptor de filtros lo comunica con aria-pressed y recuerda la elección', async () => {
    const { fixture, raiz } = await crear();
    const interruptor = raiz.querySelector<HTMLButtonElement>('.segmentado button');

    expect(interruptor?.getAttribute('aria-pressed')).toBe('true');

    interruptor?.click();
    await fixture.whenStable();

    expect(interruptor?.getAttribute('aria-pressed')).toBe('false');
    expect(interruptor?.getAttribute('aria-label')).toBe('Mostrar filtros');
    const guardado = JSON.parse(localStorage.getItem(claveDeDiseno('personas')) ?? '{}');
    expect(guardado.filtrosVisibles).toBe(false);
  });

  it('arranca con el diseño que la persona dejó guardado', async () => {
    localStorage.setItem(
      claveDeDiseno('personas'),
      JSON.stringify({ ocultas: ['empresa'], columnas: [], filtrosVisibles: false }),
    );

    const { fixture, raiz } = await crear();
    await fixture.whenStable();

    expect(
      raiz.querySelector('.segmentado button')?.getAttribute('aria-pressed'),
      'los filtros siguen ocultos como se dejaron',
    ).toBe('false');
  });

  it('usa AG Grid con el tema de serie', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('ag-grid-angular')).not.toBeNull();
  });

  describe('resaltado de lo que coincide (regla N1)', () => {
    @Component({
      imports: [Tabla],
      template: `
        <app-tabla
          etiqueta="Personas"
          [columnas]="columnas"
          [filas]="filas"
          [idFila]="idFila"
          [resaltador]="resaltador"
        />
      `,
    })
    class ConResaltado {
      columnas: ColDef<Persona>[] = [
        resaltable({ field: 'nombre', headerName: 'Nombre' }, 'NOMBRE'),
        resaltable({ field: 'empresa', headerName: 'Empresa' }, 'EMPRESA'),
        { field: 'id', headerName: 'Id' },
      ];
      filas: Persona[] = [
        { id: '1', nombre: 'ANA', empresa: 'ACME' },
        { id: '2', nombre: 'BETO', empresa: 'ACME' },
        { id: '3', nombre: 'CARLA', empresa: 'ZETA' },
      ];
      idFila = (fila: Persona): string => fila.id;
      resaltador = vi.fn<Resaltador>(async (_buscado, _campo, textos) =>
        textos.map((texto) => [{ texto, resaltado: true }]),
      );
    }

    const esperar = (ms: number): Promise<void> => new Promise((fin) => setTimeout(fin, ms));

    async function crearConResaltado() {
      TestBed.configureTestingModule({ imports: [ConResaltado] });
      const fixture = TestBed.createComponent(ConResaltado);
      await fixture.whenStable();
      const raiz = fixture.nativeElement as HTMLElement;
      const escribir = async (texto: string): Promise<void> => {
        const buscador = raiz.querySelector<HTMLInputElement>('input[type="search"]');
        buscador!.value = texto;
        buscador!.dispatchEvent(new Event('input'));
        await fixture.whenStable();
        await esperar(400);
      };
      return { fixture, anfitrion: fixture.componentInstance, escribir };
    }

    it('al buscar, pide al núcleo sólo los textos que se ven, una vez por columna resaltable', async () => {
      const { anfitrion, escribir } = await crearConResaltado();

      await escribir('ana');

      expect(anfitrion.resaltador).toHaveBeenCalledWith('ana', 'NOMBRE', ['ANA']);
      expect(anfitrion.resaltador).toHaveBeenCalledWith('ana', 'EMPRESA', ['ACME']);
      expect(anfitrion.resaltador, 'la columna Id no es resaltable').toHaveBeenCalledTimes(2);
    });

    it('sin nada escrito no pregunta al núcleo', async () => {
      const { anfitrion, escribir } = await crearConResaltado();

      await escribir('   ');

      expect(anfitrion.resaltador).not.toHaveBeenCalled();
    });

    it('si el núcleo falla, la tabla sigue funcionando sin resaltar', async () => {
      const { anfitrion, fixture, escribir } = await crearConResaltado();
      anfitrion.resaltador.mockRejectedValue(new Error('sin núcleo'));

      await escribir('ana');

      expect(anfitrion.resaltador).toHaveBeenCalled();
      expect(
        (fixture.nativeElement as HTMLElement).querySelector('.conteo')?.textContent?.trim(),
        'el filtro de la tabla no depende del resaltado',
      ).toBe('1 de 3 registros');
    });
  });
});
