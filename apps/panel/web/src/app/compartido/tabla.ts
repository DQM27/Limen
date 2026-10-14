import {
  ChangeDetectionStrategy,
  Component,
  type OnInit,
  type Signal,
  computed,
  effect,
  inject,
  input,
  signal,
} from '@angular/core';
import { MatIconModule } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import { MatTooltipModule } from '@angular/material/tooltip';
import { AG_GRID_LOCALE_ES } from '@ag-grid-community/locale';
import {
  AllCommunityModule,
  ModuleRegistry,
  colorSchemeVariable,
  themeQuartz,
  type ColDef,
  type ColumnMovedEvent,
  type ColumnResizedEvent,
  type ColumnState,
  type GetRowIdParams,
  type GridApi,
  type GridReadyEvent,
  type IRowNode,
  type ITooltipParams,
  type ModelUpdatedEvent,
  type TextMatcherParams,
} from 'ag-grid-community';
import { AgGridAngular } from 'ag-grid-angular';
import { Nucleo, type CampoDeTexto } from '../nucleo/nucleo';
import { claveDeColumna, operacionDeFiltro } from './columnas';
import { guardarDiseno, leerDiseno } from './diseno-guardado';

// Los módulos de AG Grid se registran una sola vez, aquí.
ModuleRegistry.registerModules([AllCommunityModule]);

/**
 * AG Grid elige claro u oscuro según este atributo del documento (sin él se queda en
 * claro). Se deja igual al esquema del sistema, como el resto de la página.
 */
function ajustarEsquemaDelGrid(): void {
  if (typeof document === 'undefined' || typeof matchMedia === 'undefined') {
    return;
  }
  const consulta = matchMedia('(prefers-color-scheme: dark)');
  const aplicar = (): void => {
    document.body.dataset['agThemeMode'] = consulta.matches ? 'dark' : 'light';
  };
  aplicar();
  consulta.addEventListener?.('change', aplicar);
}

ajustarEsquemaDelGrid();

/** Lo que las celdas resaltadas leen de la tabla (ver `compartido/resaltado.ts`). */
export interface ContextoDeTabla {
  buscado: () => string;
}

/**
 * La tabla de todo el panel, igual que la de la app de escritorio: AG Grid con su aspecto de
 * serie (claro u oscuro según el sistema, con los colores de Tokyo Night), una sola fila de
 * herramientas y textos en español. Cada pantalla le da sus columnas y sus filas y puede poner
 * sus propios controles a la izquierda (`<... controles>`) y a la derecha (`<... acciones>`).
 *
 * Es el único lugar donde viven las piezas que toda grilla trae: el buscador, los filtros por
 * columna (con su botón para mostrarlos u ocultarlos), el ajuste de anchos en los dos sentidos y
 * el selector de columnas. Las pantallas las heredan sin repetirlas.
 *
 * El buscador, si la pantalla da `camposDeBusqueda`, y los filtros de texto de cada columna son
 * del núcleo (regla N2): la tabla no compara, no pliega ni parte por palabras; pregunta y aplica
 * lo que responde. Con un `id`, el diseño que cada persona le da (columnas, anchos, orden,
 * filtros) se guarda en este navegador.
 */
@Component({
  selector: 'app-tabla',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [AgGridAngular, MatIconModule, MatMenuModule, MatTooltipModule],
  templateUrl: './tabla.html',
  styleUrl: './tabla.scss',
})
export class Tabla<T> implements OnInit {
  private readonly nucleo = inject(Nucleo);

  /** Con un identificador, el diseño de la tabla se guarda en este navegador. */
  readonly id = input<string>();
  /** Sin filtros por columna, la tabla no ofrece el botón de mostrarlos. */
  readonly filtrosPorColumna = input(true);

  /** Cómo se llama esta tabla ("Equipos"): la anuncian los lectores de pantalla. */
  readonly etiqueta = input.required<string>();
  readonly columnas = input.required<ColDef<T>[]>();
  readonly filas = input.required<T[]>();
  readonly cargando = input(false);
  readonly mensajeVacio = input('Sin resultados');
  readonly marcador = input('Buscar…');
  /** Identificador estable de cada fila: AG Grid actualiza la fila en vez de repintar todo. */
  readonly idFila = input.required<(fila: T) => string>();
  /** Los textos de una fila que el buscador mira. Sin esto, la tabla no tiene buscador. */
  readonly camposDeBusqueda = input<(fila: T) => CampoDeTexto[]>();

  protected readonly textos = AG_GRID_LOCALE_ES;
  /** Quartz de serie con los colores de la app (los tokens de Tokyo Night). */
  protected readonly tema = themeQuartz.withPart(colorSchemeVariable).withParams({
    backgroundColor: 'var(--mat-sys-surface)',
    foregroundColor: 'var(--mat-sys-on-surface)',
    headerBackgroundColor: 'var(--mat-sys-surface-container)',
    borderColor: 'var(--mat-sys-outline-variant)',
    accentColor: 'var(--mat-sys-primary)',
    chromeBackgroundColor: 'var(--mat-sys-surface-container-low)',
  });
  protected readonly buscado = signal('');
  protected readonly contexto: ContextoDeTabla = { buscado: () => this.buscado() };
  private api: GridApi<T> | null = null;
  /** Cuántas filas se ven con el buscador y los filtros puestos; `null` hasta que llegan las filas. */
  protected readonly visibles = signal<number | null>(null);
  protected readonly ocultas = signal<ReadonlySet<string>>(new Set());
  protected readonly filtrosVisibles = signal(true);
  /** Orden, ancho y fijado de las columnas: lo último guardado o lo que haya hoy. */
  private columnasGuardadas: ColumnState[] = [];

  /**
   * El filtro de texto de una columna lo decide el núcleo (N2). AG Grid pasaría el texto ya en
   * minúsculas; el núcleo recibe el original, porque la pantalla no transforma nada antes de que
   * él decida.
   */
  private readonly coincideFiltroDeColumna = (parametros: TextMatcherParams): boolean =>
    parametros.filterText === null ||
    this.nucleo.celdaCumple(
      operacionDeFiltro(parametros.filterOption),
      String(parametros.value ?? ''),
      parametros.filterText,
    );

  protected readonly columnaPorDefecto = computed<ColDef<T>>(() => ({
    sortable: true,
    resizable: true,
    minWidth: 90,
    filter: this.filtrosPorColumna(),
    floatingFilter: this.filtrosPorColumna() && this.filtrosVisibles(),
    filterParams: {
      textMatcher: this.coincideFiltroDeColumna,
      textFormatter: (texto: string) => texto,
    },
    tooltipValueGetter: textoDeTooltip,
  }));

  /**
   * Las columnas con su visibilidad. Una columna que muestra un valor con formato (una fecha)
   * se filtra por lo que se ve, no por el dato crudo.
   */
  protected readonly columnasConVisibilidad: Signal<ColDef<T>[]> = computed(() =>
    this.columnas().map((columna) => {
      const clave = claveDeColumna(columna);
      const conFormato =
        columna.valueFormatter && !columna.filterValueGetter && clave
          ? {
              filterValueGetter: (p: { api: GridApi<T>; node: IRowNode<T> | null }): string => {
                const valor: unknown = p.node
                  ? p.api.getCellValue({ rowNode: p.node, colKey: clave, useFormatter: true })
                  : '';
                return valor === null || valor === undefined ? '' : String(valor);
              },
            }
          : {};
      return clave ? { ...columna, ...conFormato, hide: this.ocultas().has(clave) } : columna;
    }),
  );

  /**
   * Las columnas que se pueden mostrar u ocultar, para el menú. Una columna con `lockVisible`
   * (por ejemplo, la del botón de una acción) no se ofrece: siempre se ve.
   */
  protected readonly columnasDelMenu: Signal<{ clave: string; titulo: string }[]> = computed(() =>
    this.columnas().flatMap((columna) => {
      const clave = claveDeColumna(columna);
      return clave && !columna.lockVisible ? [{ clave, titulo: columna.headerName ?? clave }] : [];
    }),
  );

  /** El ID de las filas que coinciden con el buscador; `null` si no hay nada que buscar. */
  private readonly coinciden = computed<ReadonlySet<string> | null>(() => {
    const buscado = this.buscado();
    const campos = this.camposDeBusqueda();
    if (buscado.trim() === '' || !campos) {
      return null;
    }
    const filas = this.filas();
    const respuestas = this.nucleo.filasQueCoinciden(buscado, filas.map(campos));
    const id = this.idFila();
    return new Set(filas.filter((_, i) => respuestas[i]).map(id));
  });

  protected readonly conteo = computed(() => {
    const total = this.filas().length;
    const visibles = this.visibles() ?? total;
    if (visibles !== total) {
      return `${visibles} de ${total} filas`;
    }
    return total === 1 ? '1 fila' : `${total} filas`;
  });

  protected readonly obtenerIdFila = computed(() => {
    const id = this.idFila();
    return (parametros: GetRowIdParams<T>) => id(parametros.data);
  });

  constructor() {
    // Cuando cambia lo escrito o las filas, la grilla vuelve a preguntar qué se ve y repinta
    // el resaltado de las celdas.
    effect(() => {
      this.coinciden();
      this.filas();
      this.api?.onFilterChanged();
      this.api?.refreshCells({ force: true });
    });
  }

  protected readonly hayBusqueda = (): boolean => this.coinciden() !== null;

  protected readonly pasaLaBusqueda = (nodo: IRowNode<T>): boolean => {
    const coinciden = this.coinciden();
    return (
      coinciden === null || (nodo.data !== undefined && coinciden.has(this.idFila()(nodo.data)))
    );
  };

  protected escribir(evento: Event): void {
    // Lo escrito se ve como lo dice el núcleo (mayúsculas, sin tildes: regla N3); el cursor se
    // queda donde estaba.
    const campo = evento.target as HTMLInputElement;
    const cursor = campo.selectionStart;
    const queda = this.nucleo.buscadorAlEscribir(campo.value);
    if (queda !== campo.value) {
      campo.value = queda;
      campo.setSelectionRange(cursor, cursor);
    }
    this.buscado.set(queda);
  }

  /** Lo guardado se lee al crear la tabla: no depende de que la grilla esté lista. */
  ngOnInit(): void {
    const guardado = leerDiseno(this.id());
    if (!guardado) {
      return;
    }
    this.ocultas.set(new Set(guardado.ocultas));
    this.filtrosVisibles.set(guardado.filtrosVisibles);
    this.columnasGuardadas = guardado.columnas;
  }

  protected alIniciar(evento: GridReadyEvent<T>): void {
    this.api = evento.api;
    if (this.columnasGuardadas.length > 0) {
      evento.api.applyColumnState({ state: this.columnasGuardadas, applyOrder: true });
    }
    this.visibles.set(evento.api.getDisplayedRowCount());
  }

  protected alActualizarFilas(evento: ModelUpdatedEvent<T>): void {
    this.visibles.set(evento.api.getDisplayedRowCount());
  }

  protected alMoverColumna(evento: ColumnMovedEvent<T>): void {
    if (evento.finished) {
      this.guardar();
    }
  }

  protected alRedimensionarColumna(evento: ColumnResizedEvent<T>): void {
    if (evento.finished) {
      this.guardar();
    }
  }

  protected guardarOrden(): void {
    this.guardar();
  }

  protected alternarFiltros(): void {
    this.filtrosVisibles.update((visibles) => !visibles);
    this.guardar();
  }

  protected alternarColumna(clave: string): void {
    this.ocultas.update((actuales) => {
      const siguientes = new Set(actuales);
      if (!siguientes.delete(clave)) {
        siguientes.add(clave);
      }
      return siguientes;
    });
    this.guardar();
  }

  /**
   * Cada columna al ancho de su contenido. Se les quita el reparto proporcional (`flex`), porque
   * si no, AG Grid lo vuelve a aplicar encima.
   */
  protected ajustarAnchos(): void {
    const api = this.api;
    if (!api) {
      return;
    }
    api.applyColumnState({
      state: api.getColumnState().map((columna) => ({ colId: columna.colId, flex: null })),
    });
    api.autoSizeAllColumns();
    this.guardar();
  }

  /** Vuelve al reparto de anchos con el que la pantalla definió sus columnas. */
  protected restablecerAnchos(): void {
    const api = this.api;
    if (!api) {
      return;
    }
    api.applyColumnState({
      state: this.columnas().flatMap((columna) => {
        const colId = claveDeColumna(columna);
        return colId
          ? [{ colId, flex: columna.flex ?? null, width: columna.flex ? undefined : columna.width }]
          : [];
      }),
    });
    this.guardar();
  }

  private guardar(): void {
    if (this.api) {
      // `hide` no se guarda: `ocultas` es la única fuente de verdad.
      this.columnasGuardadas = this.api
        .getColumnState()
        .map(({ hide: _oculta, ...resto }) => resto);
    }
    guardarDiseno(this.id(), {
      ocultas: [...this.ocultas()],
      columnas: this.columnasGuardadas,
      filtrosVisibles: this.filtrosVisibles(),
    });
  }
}

/** Texto completo al pasar el mouse, sólo para celdas de texto o número. */
function textoDeTooltip(parametros: ITooltipParams): string | undefined {
  const valor: unknown = parametros.valueFormatted ?? parametros.value;
  return typeof valor === 'string' || typeof valor === 'number' ? String(valor) : undefined;
}
