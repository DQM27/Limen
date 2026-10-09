import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  output,
  signal,
  type OnInit,
  type Signal,
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
  type ColTypeDefs,
  type ColumnMovedEvent,
  type ColumnState,
  type ColumnResizedEvent,
  type GetRowIdParams,
  type GridApi,
  type GridReadyEvent,
  type ModelUpdatedEvent,
  type RowClassParams,
  type SelectionChangedEvent,
} from 'ag-grid-community';
import { AgGridAngular } from 'ag-grid-angular';
import { claveDeColumna, textoDeTooltip, tiposDeColumna } from './columnas';
import { guardarDiseno, leerDiseno } from './diseno-guardado';
import { analizarBusqueda, coincideBusqueda, coincideFiltroDeColumna } from './pliegue';

// Los módulos de AG Grid se registran una sola vez, aquí. Hoy todos los de la
// edición Community; si el tamaño del paquete llegara a importar, se acota en
// este único lugar.
ModuleRegistry.registerModules([AllCommunityModule]);

/**
 * La tabla de toda la app. Cada pantalla le da sus columnas y sus filas, y
 * hereda el diseño unificado: AG Grid con su aspecto de serie (claro u oscuro
 * según el sistema), una sola fila de herramientas, textos en español,
 * buscador y filtros por columna que no distinguen tildes, filtros de fecha y
 * número, orden, columnas visibles, anchos, selección múltiple opcional,
 * estados de carga y vacío, y el diseño que cada persona le da queda guardado.
 * Un cambio de diseño se hace aquí y lo heredan todas.
 *
 * La pantalla puede poner sus propios controles a la izquierda
 * (`<... controles>`) y a la derecha (`<... acciones>`) de la fila de
 * herramientas.
 */
@Component({
  selector: 'app-tabla',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [AgGridAngular, MatIconModule, MatMenuModule, MatTooltipModule],
  templateUrl: './tabla.html',
  styleUrl: './tabla.scss',
})
export class Tabla<T> implements OnInit {
  /** Cómo se llama esta tabla ("Contratistas"): la anuncian los lectores de pantalla. */
  readonly etiqueta = input.required<string>();
  readonly columnas = input.required<ColDef<T>[]>();
  readonly filas = input.required<T[]>();
  readonly cargando = input(false);
  readonly marcador = input('Buscar…');
  readonly mensajeVacio = input('Sin resultados');
  /** Identificador de esta tabla: con él se guarda su diseño en este equipo. */
  readonly id = input<string>();
  /** Identificador estable de cada fila; mantiene la selección al refrescar. */
  readonly idFila = input<(fila: T) => string>();
  readonly filtrosPorColumna = input(true);
  readonly seleccionMultiple = input(false);
  readonly claseFila = input<(fila: T) => string | undefined>();

  readonly seleccionCambia = output<T[]>();
  readonly filaDobleClic = output<T>();

  protected readonly textos = AG_GRID_LOCALE_ES;
  /** Quartz de serie; el esquema (claro u oscuro) sigue al del sistema. */
  protected readonly tema = themeQuartz.withPart(colorSchemeVariable);
  protected readonly tiposDeColumna: ColTypeDefs<T> = tiposDeColumna as ColTypeDefs<T>;
  protected readonly analizarBusqueda = analizarBusqueda;
  protected readonly coincideBusqueda = coincideBusqueda;

  protected readonly filtro = signal('');
  protected readonly ocultas = signal<ReadonlySet<string>>(new Set());
  protected readonly filtrosVisibles = signal(true);
  /** Cuántas filas se ven con el filtro puesto. */
  protected readonly visibles = signal<number | null>(null);

  private api: GridApi<T> | null = null;
  /** Orden, ancho y fijado de las columnas: lo último guardado o lo que haya hoy. */
  private columnasGuardadas: ColumnState[] = [];

  protected readonly columnaPorDefecto: Signal<ColDef<T>> = computed(() => ({
    sortable: true,
    resizable: true,
    minWidth: 90,
    flex: 1,
    filter: true,
    floatingFilter: this.filtrosPorColumna() && this.filtrosVisibles(),
    filterParams: { textMatcher: coincideFiltroDeColumna },
    tooltipValueGetter: textoDeTooltip,
    ...(this.idFila() ? { enableCellChangeFlash: true } : {}),
  }));

  protected readonly columnasConVisibilidad: Signal<ColDef<T>[]> = computed(() =>
    this.columnas().map((columna) => {
      const clave = claveDeColumna(columna);
      return clave ? { ...columna, hide: this.ocultas().has(clave) } : columna;
    }),
  );

  /** Las columnas que se pueden mostrar u ocultar, para el menú. */
  protected readonly columnasDelMenu: Signal<{ clave: string; titulo: string }[]> = computed(() =>
    this.columnas().flatMap((columna) => {
      const clave = claveDeColumna(columna);
      return clave ? [{ clave, titulo: columna.headerName ?? clave }] : [];
    }),
  );

  protected readonly total: Signal<number> = computed(() => this.filas().length);
  protected readonly conteo: Signal<string> = computed(() => {
    const total = this.total();
    const visibles = this.visibles() ?? total;
    const palabra = total === 1 ? 'registro' : 'registros';
    return visibles === total ? `${total} ${palabra}` : `${visibles} de ${total} ${palabra}`;
  });

  protected readonly obtenerIdFila: Signal<((p: GetRowIdParams<T>) => string) | undefined> =
    computed(() => {
      const id = this.idFila();
      return id ? (parametros: GetRowIdParams<T>): string => id(parametros.data) : undefined;
    });

  protected readonly obtenerClaseFila: Signal<
    ((p: RowClassParams<T>) => string | undefined) | undefined
  > = computed(() => {
    const clase = this.claseFila();
    return clase
      ? (p: RowClassParams<T>): string | undefined => (p.data ? clase(p.data) : undefined)
      : undefined;
  });

  protected readonly seleccion = computed(() =>
    this.seleccionMultiple()
      ? ({ mode: 'multiRow', checkboxes: true, headerCheckbox: true } as const)
      : undefined,
  );

  protected escribirFiltro(evento: Event): void {
    this.filtro.set((evento.target as HTMLInputElement).value);
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

  protected alCambiarSeleccion(evento: SelectionChangedEvent<T>): void {
    this.seleccionCambia.emit(evento.api.getSelectedRows());
  }

  protected alDobleClic(fila: T | undefined): void {
    if (fila) {
      this.filaDobleClic.emit(fila);
    }
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
   * Cada columna al ancho de su contenido. Se les quita el reparto
   * proporcional (`flex`), porque si no, AG Grid lo vuelve a aplicar encima.
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

/**
 * AG Grid elige claro u oscuro según este atributo del documento. Se deja
 * igual al esquema del sistema, como el resto de la app.
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
