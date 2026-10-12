import { Component, type DebugElement } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import type { ColDef, GridApi } from 'ag-grid-community';
import { AgGridAngular } from 'ag-grid-angular';
import type { ReglasDeBusqueda } from '../../nucleo/busqueda';
import { claveDeDiseno } from './diseno-guardado';
import { resaltable } from './resaltado';
import { Tabla } from './tabla';

/**
 * Un doble de las reglas del núcleo: sólo sirve para probar que la tabla
 * pregunta y aplica lo que responde. No es una regla (esas se prueban en el
 * dominio).
 */
function reglasDePrueba() {
  return {
    resaltar: vi.fn<ReglasDeBusqueda['resaltar']>(async (_buscado, _campo, textos) =>
      textos.map((texto) => [{ texto, resaltado: true }]),
    ),
    filasQueCoinciden: vi.fn<ReglasDeBusqueda['filasQueCoinciden']>(async (buscado, filas) =>
      filas.map((fila) => fila.some((c) => c.texto.toLowerCase().includes(buscado.toLowerCase()))),
    ),
    celdasQueCumplen: vi.fn<ReglasDeBusqueda['celdasQueCumplen']>(async (pedidos) =>
      pedidos.map((p) => p.celda.toLowerCase().includes(p.filtro.toLowerCase())),
    ),
  } satisfies ReglasDeBusqueda;
}

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
      [reglas]="reglas"
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
  reglas = reglasDePrueba();
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

  describe('búsqueda y resaltado con las reglas del núcleo (N1 y N2)', () => {
    @Component({
      imports: [Tabla],
      template: `
        <app-tabla
          etiqueta="Personas"
          [columnas]="columnas"
          [filas]="filas"
          [idFila]="idFila"
          [reglas]="reglas"
        />
      `,
    })
    class ConReglas {
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
      reglas = reglasDePrueba();
    }

    const esperar = (ms: number): Promise<void> => new Promise((fin) => setTimeout(fin, ms));

    async function crearConReglas() {
      TestBed.configureTestingModule({ imports: [ConReglas] });
      const fixture = TestBed.createComponent(ConReglas);
      await fixture.whenStable();
      const raiz = fixture.nativeElement as HTMLElement;
      const escribir = async (texto: string, espera = 400): Promise<void> => {
        const buscador = raiz.querySelector<HTMLInputElement>('input[type="search"]');
        buscador!.value = texto;
        buscador!.dispatchEvent(new Event('input'));
        await fixture.whenStable();
        await esperar(espera);
      };
      const conteo = (): string | undefined => raiz.querySelector('.conteo')?.textContent?.trim();
      const api = (): GridApi<Persona> => {
        const rejilla: DebugElement = fixture.debugElement.query(By.directive(AgGridAngular));
        return (rejilla.componentInstance as AgGridAngular<Persona>).api;
      };
      return { fixture, anfitrion: fixture.componentInstance, escribir, conteo, api };
    }

    it('el buscador muestra las filas que el núcleo dice que coinciden', async () => {
      const { anfitrion, escribir, conteo } = await crearConReglas();

      await escribir('ana');

      expect(anfitrion.reglas.filasQueCoinciden).toHaveBeenCalledWith('ana', [
        [
          { campo: 'NOMBRE', texto: 'ANA' },
          { campo: 'EMPRESA', texto: 'ACME' },
        ],
        [
          { campo: 'NOMBRE', texto: 'BETO' },
          { campo: 'EMPRESA', texto: 'ACME' },
        ],
        [
          { campo: 'NOMBRE', texto: 'CARLA' },
          { campo: 'EMPRESA', texto: 'ZETA' },
        ],
      ]);
      expect(conteo(), 'sólo las que respondió el núcleo').toBe('1 de 3 registros');
    });

    it('sólo busca en las columnas que declararon su campo', async () => {
      const { anfitrion, escribir } = await crearConReglas();

      await escribir('ana');

      const filas = vi.mocked(anfitrion.reglas.filasQueCoinciden).mock.calls[0]?.[1] ?? [];
      expect(
        filas.every((fila) => fila.length === 2),
        'la columna Id no es parte de la búsqueda',
      ).toBe(true);
    });

    it('al borrar el buscador vuelven todas las filas y no se pregunta al núcleo', async () => {
      const { anfitrion, escribir, conteo } = await crearConReglas();
      await escribir('ana');
      expect(conteo()).toBe('1 de 3 registros');
      const preguntas = vi.mocked(anfitrion.reglas.filasQueCoinciden).mock.calls.length;

      await escribir('  ');

      expect(conteo()).toBe('3 registros');
      expect(vi.mocked(anfitrion.reglas.filasQueCoinciden)).toHaveBeenCalledTimes(preguntas);
    });

    it('pide el resaltado de lo que se ve, una vez por columna resaltable', async () => {
      const { anfitrion, escribir } = await crearConReglas();

      await escribir('ana');

      const { resaltar } = anfitrion.reglas;
      expect(resaltar).toHaveBeenCalledWith('ana', 'NOMBRE', ['ANA']);
      expect(resaltar).toHaveBeenCalledWith('ana', 'EMPRESA', ['ACME']);
      const campos = new Set(vi.mocked(resaltar).mock.calls.map(([, campo]) => campo));
      expect([...campos].sort(), 'la columna Id no se resalta').toEqual(['EMPRESA', 'NOMBRE']);
    });

    it('si el núcleo falla, la tabla muestra todo en vez de ocultar filas', async () => {
      const { anfitrion, escribir, conteo } = await crearConReglas();
      anfitrion.reglas.filasQueCoinciden.mockRejectedValue(new Error('sin núcleo'));
      anfitrion.reglas.resaltar.mockRejectedValue(new Error('sin núcleo'));

      await escribir('ana');

      expect(anfitrion.reglas.filasQueCoinciden).toHaveBeenCalled();
      expect(conteo(), 'sin respuesta no se esconde nada').toBe('3 registros');
    });

    it('sólo vale la última respuesta: una anterior que llega tarde se descarta', async () => {
      const { anfitrion, escribir, conteo } = await crearConReglas();
      let responderLaPrimera: (valor: boolean[]) => void = () => undefined;
      anfitrion.reglas.filasQueCoinciden
        .mockImplementationOnce(
          () => new Promise<boolean[]>((resolver) => (responderLaPrimera = resolver)),
        )
        .mockImplementationOnce(async (_buscado, filas) => filas.map((_, i) => i === 2));

      await escribir('a', 250);
      await escribir('ar', 400);
      expect(conteo(), 'ganó la segunda pregunta').toBe('1 de 3 registros');

      responderLaPrimera([true, true, true]);
      await esperar(100);

      expect(conteo(), 'la primera, tardía, no pisa a la segunda').toBe('1 de 3 registros');
    });

    it('los filtros de columna le preguntan al núcleo celda por celda, en un solo viaje', async () => {
      const { anfitrion, api, conteo } = await crearConReglas();

      api().setFilterModel({ nombre: { filterType: 'text', type: 'contains', filter: 'ana' } });
      await esperar(300);

      const { celdasQueCumplen } = anfitrion.reglas;
      expect(celdasQueCumplen).toHaveBeenCalledTimes(1);
      expect(celdasQueCumplen).toHaveBeenCalledWith([
        { operacion: 'CONTIENE', celda: 'ANA', filtro: 'ana' },
        { operacion: 'CONTIENE', celda: 'BETO', filtro: 'ana' },
        { operacion: 'CONTIENE', celda: 'CARLA', filtro: 'ana' },
      ]);
      expect(conteo(), 'y la grilla filtra con lo que respondió').toBe('1 de 3 registros');
    });

    it('cada opción del filtro de AG Grid llega al núcleo con su código', async () => {
      const { anfitrion, api } = await crearConReglas();

      api().setFilterModel({ nombre: { filterType: 'text', type: 'startsWith', filter: 'be' } });
      await esperar(300);

      expect(anfitrion.reglas.celdasQueCumplen).toHaveBeenCalledWith(
        expect.arrayContaining([{ operacion: 'EMPIEZA_CON', celda: 'BETO', filtro: 'be' }]),
      );
    });
  });
});
