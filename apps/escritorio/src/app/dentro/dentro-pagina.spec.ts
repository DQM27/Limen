import { TestBed } from '@angular/core/testing';
import { MatDialog } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import type { ColDef } from 'ag-grid-community';
import type { PersonaAdentro } from '../nucleo/tipos';
import { DentroPagina } from './dentro-pagina';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const persona = (id: string, nombre: string, cambios: Partial<PersonaAdentro> = {}) =>
  ({
    via: 'CONTRATISTA',
    ingreso_id: id,
    identidad: `1${id}`,
    nombre,
    procedencia: 'ACME S.A.',
    medio: 'A_PIE',
    placa: null,
    gafete: 3,
    sin_gafete: false,
    entrada: '2026-10-09T14:00:00Z',
    entrada_por: { id: 'o1', nombre: 'OPERADOR UNO' },
    demasiado_tiempo: false,
    ...cambios,
  }) satisfies PersonaAdentro;

/** Lo que la pantalla expone para probarla sin simular el ratón de la grilla. */
interface Interno {
  visibles(): PersonaAdentro[];
  seleccionadas: { set(filas: PersonaAdentro[]): void };
  columnas: ColDef<PersonaAdentro>[];
  claseFila(fila: PersonaAdentro): string | undefined;
  cambiarFiltro(filtro: 'todos' | 'con' | 'sin'): void;
  salida(persona: PersonaAdentro): Promise<void>;
  salidaMasiva(): void;
  actualizar(): void;
}

async function crear() {
  const fixture = TestBed.createComponent(DentroPagina);
  await fixture.whenStable();
  return {
    fixture,
    raiz: fixture.nativeElement as HTMLElement,
    interno: fixture.componentInstance as unknown as Interno,
  };
}

const llamadas = (comando: string) =>
  vi.mocked(invoke).mock.calls.filter(([nombre]) => nombre === comando);

describe('pantalla Dentro', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    document.querySelectorAll('.cdk-overlay-container > *').forEach((e) => e.remove());
  });

  it('pide quién está adentro y muestra cuántos son', async () => {
    vi.mocked(invoke).mockResolvedValue([persona('1', 'ANA'), persona('2', 'BETO')]);

    const { raiz } = await crear();

    expect(invoke).toHaveBeenCalledWith('dentro', undefined);
    expect(raiz.querySelector('h1')?.textContent).toBe('Dentro');
    expect(raiz.querySelector('.conteo')?.textContent?.trim()).toBe('2 registros');
  });

  it('si el núcleo falla, muestra su mensaje tal cual', async () => {
    vi.mocked(invoke).mockRejectedValue({
      tipo: 'negocio',
      codigo: 'sin_sesion',
      mensaje: 'Inicie sesión para continuar',
    });

    const { raiz } = await crear();

    expect(raiz.querySelector('[role="alert"]')?.textContent).toContain(
      'Inicie sesión para continuar',
    );
  });

  it('el filtro separa a quien tiene gafete de quien no, sin volver a pedir nada', async () => {
    vi.mocked(invoke).mockResolvedValue([
      persona('1', 'ANA'),
      persona('2', 'BETO', { gafete: null, sin_gafete: true }),
    ]);
    const { interno } = await crear();

    interno.cambiarFiltro('con');
    expect(interno.visibles().map((p) => p.nombre)).toEqual(['ANA']);
    interno.cambiarFiltro('sin');
    expect(interno.visibles().map((p) => p.nombre)).toEqual(['BETO']);
    interno.cambiarFiltro('todos');
    expect(interno.visibles()).toHaveLength(2);
    expect(llamadas('dentro')).toHaveLength(1);
  });

  it('resalta a quien el núcleo dice que lleva demasiado tiempo adentro', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { interno } = await crear();

    expect(interno.claseFila(persona('1', 'ANA', { demasiado_tiempo: true }))).toBe(
      'fila-demasiado-tiempo',
    );
    expect(interno.claseFila(persona('2', 'BETO'))).toBeUndefined();
  });

  it('la salida de una persona se registra con su vía y su ingreso, y la lista se recarga', async () => {
    vi.mocked(invoke).mockResolvedValue([persona('1', 'ANA')]);
    const { interno } = await crear();

    await interno.salida(persona('1', 'ANA', { via: 'PROVEEDOR' }));

    expect(invoke).toHaveBeenCalledWith('registrar_salida', { via: 'PROVEEDOR', ingresoId: '1' });
    expect(llamadas('dentro')).toHaveLength(2);
  });

  it('el botón "Salida" de la fila registra la salida de esa persona', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { interno } = await crear();
    const columna = interno.columnas.find((c) => c.colId === 'accion');
    const boton = document.createElement('button');
    boton.className = 'boton-celda';

    (columna?.onCellClicked as (e: unknown) => void)({
      data: persona('7', 'LUIS'),
      event: { target: boton },
    });
    (columna?.onCellClicked as (e: unknown) => void)({
      data: persona('8', 'OTRO'),
      event: { target: document.createElement('div') },
    });
    await vi.waitFor(() => expect(llamadas('registrar_salida')).toHaveLength(1));

    expect(invoke).toHaveBeenCalledWith('registrar_salida', { via: 'CONTRATISTA', ingresoId: '7' });
  });

  it('si la salida se rechaza, no se rompe y la lista se recarga igual', async () => {
    vi.mocked(invoke).mockImplementation((comando: string) =>
      comando === 'registrar_salida'
        ? Promise.reject({
            tipo: 'negocio',
            codigo: 'ya_salio',
            mensaje: 'Ya salió',
            campo: null,
          })
        : Promise.resolve([]),
    );
    const { interno } = await crear();

    await interno.salida(persona('1', 'ANA'));

    expect(llamadas('dentro')).toHaveLength(2);
  });

  it('la salida de varias pide confirmar y registra cada una', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { interno } = await crear();
    interno.seleccionadas.set([persona('1', 'ANA'), persona('2', 'BETO', { via: 'KOF' })]);

    interno.salidaMasiva();
    await vi.waitFor(() =>
      expect(document.querySelector('app-dialogo-confirmar .mensaje')?.textContent).toContain(
        '2 personas',
      ),
    );
    const modal = document.querySelector('app-dialogo-confirmar');
    expect(modal?.textContent).toContain('¿Registrar la salida de 2 personas?');
    expect(llamadas('registrar_salida')).toHaveLength(0);

    [...(modal?.querySelectorAll('button') ?? [])]
      .find((b) => b.textContent?.includes('Registrar salida'))
      ?.click();
    await vi.waitFor(() => expect(llamadas('registrar_salida')).toHaveLength(2));

    expect(invoke).toHaveBeenCalledWith('registrar_salida', { via: 'CONTRATISTA', ingresoId: '1' });
    expect(invoke).toHaveBeenCalledWith('registrar_salida', { via: 'KOF', ingresoId: '2' });
    TestBed.inject(MatDialog).closeAll();
  });

  it('cancelar la confirmación no registra nada', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { interno } = await crear();
    interno.seleccionadas.set([persona('1', 'ANA')]);

    interno.salidaMasiva();
    await vi.waitFor(() =>
      expect(document.querySelector('app-dialogo-confirmar .mensaje')?.textContent).toContain(
        'ANA?',
      ),
    );
    expect(document.querySelector('app-dialogo-confirmar')?.textContent).toContain(
      '¿Registrar la salida de ANA?',
    );
    [...document.querySelectorAll<HTMLButtonElement>('app-dialogo-confirmar button')]
      .find((b) => b.textContent?.includes('Cancelar'))
      ?.click();

    expect(llamadas('registrar_salida')).toHaveLength(0);
    TestBed.inject(MatDialog).closeAll();
  });

  it('sin nadie seleccionado, la salida de varias no hace nada', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { interno } = await crear();

    interno.salidaMasiva();

    expect(document.querySelector('app-dialogo-confirmar')).toBeNull();
  });

  it('actualizar vuelve a pedir quién está adentro', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { fixture, interno } = await crear();

    interno.actualizar();
    await fixture.whenStable();

    expect(llamadas('dentro')).toHaveLength(2);
  });
});
