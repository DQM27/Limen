import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
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
  type IRowNode,
  type ModelUpdatedEvent,
  type RowClassParams,
  type SelectionChangedEvent,
  type TextMatcherParams,
} from 'ag-grid-community';
import { AgGridAngular } from 'ag-grid-angular';
import { Atajo } from '../atajo';
import { claveDeColumna, textoDeTooltip, tiposDeColumna } from './columnas';
import { guardarDiseno, leerDiseno } from './diseno-guardado';
import type { ReglasDeBusqueda } from '../../nucleo/busqueda';
import type { CampoDeTexto, CampoResaltable, Segmento } from '../../nucleo/tipos';
import { DecisionesDeColumna, operacionDeFiltro } from './decisiones-de-columna';
import {
  MAXIMO_TEXTOS_POR_COLUMNA,
  campoResaltable,
  claveDeTexto,
  type ContextoDeResaltado,
} from './resaltado';

/** Cuánto se espera tras teclear antes de preguntarle al núcleo. */
const RETARDO_DE_BUSQUEDA_MS = 150;

// Los módulos de AG Grid se registran una sola vez, aquí. Hoy todos los de la
// edición Community; si el tamaño del paquete llegara a importar, se acota en
// este único lugar.
ModuleRegistry.registerModules([AllCommunityModule]);

/**
 * La tabla de toda la app. Cada pantalla le da sus columnas y sus filas, y
 * hereda el diseño unificado: AG Grid con su aspecto de serie (claro u oscuro
 * según el sistema), una sola fila de herramientas, textos en español,
 * buscador y filtros de texto por columna (decididos por el núcleo, reglas N1
 * y N2), filtros de fecha y número, orden, columnas visibles, anchos, selección múltiple opcional,
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
  imports: [AgGridAngular, Atajo, MatIconModule, MatMenuModule, MatTooltipModule],
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
  /**
   * Las reglas de búsqueda del núcleo (N1 y N2): qué filas coinciden con el
   * buscador, qué celdas cumplen un filtro de columna y qué resaltar. La
   * tabla no decide ninguna. El buscador y el resaltado miran las columnas
   * marcadas con `resaltable()`.
   */
  readonly reglas = input.required<ReglasDeBusqueda>();

  readonly seleccionCambia = output<T[]>();
  readonly filaDobleClic = output<T>();

  protected readonly textos = AG_GRID_LOCALE_ES;
  /**
   * Quartz de serie; el esquema (claro u oscuro) sigue al del sistema. Sólo se
   * le dan los colores de la app (los tokens de Tokyo Night): su forma no se toca.
   */
  protected readonly tema = themeQuartz.withPart(colorSchemeVariable).withParams({
    backgroundColor: 'var(--mat-sys-surface)',
    foregroundColor: 'var(--mat-sys-on-surface)',
    headerBackgroundColor: 'var(--mat-sys-surface-container)',
    borderColor: 'var(--mat-sys-outline-variant)',
    accentColor: 'var(--mat-sys-primary)',
    chromeBackgroundColor: 'var(--mat-sys-surface-container-low)',
  });
  protected readonly tiposDeColumna: ColTypeDefs<T> = tiposDeColumna as ColTypeDefs<T>;

  protected readonly filtro = signal('');
  protected readonly ocultas = signal<ReadonlySet<string>>(new Set());
  protected readonly filtrosVisibles = signal(true);
  /** Cuántas filas se ven con el filtro puesto. */
  protected readonly visibles = signal<number | null>(null);

  /** Los segmentos que el núcleo devolvió para lo que hay en pantalla. */
  private readonly resaltados = signal<ReadonlyMap<string, readonly Segmento[]>>(new Map());
  /** Lo que ven las celdas resaltables (el `context` del grid). */
  protected readonly contexto: ContextoDeResaltado = {
    segmentosDe: (campo, texto) => this.resaltados().get(claveDeTexto(campo, texto)),
  };
  /** Cada pedido nuevo invalida los anteriores: sólo vale el último. */
  private pedidoDeResaltado = 0;
  private temporizadorDeResaltado: ReturnType<typeof setTimeout> | undefined;

  /**
   * Las filas que el núcleo dice que pasan el buscador (`null`: no hay
   * búsqueda). Se identifican por su `idFila` si la hay, o por la fila misma.
   */
  private filasQuePasan: ReadonlySet<unknown> | null = null;
  private pedidoDeBusqueda = 0;
  private temporizadorDeBusqueda: ReturnType<typeof setTimeout> | undefined;

  /** AG Grid filtra por lo que decidió el núcleo, no por una regla propia. */
  protected readonly hayBusqueda = (): boolean => this.filasQuePasan !== null;
  protected readonly pasaLaBusqueda = (nodo: IRowNode<T>): boolean =>
    this.filasQuePasan === null ||
    (nodo.data !== undefined && this.filasQuePasan.has(this.claveDeFila(nodo.data)));

  /** Los filtros de texto de cada columna: los decide el núcleo (N2). */
  private readonly decisiones = new DecisionesDeColumna(
    (pedidos) => this.reglas().celdasQueCumplen(pedidos),
    () => this.api?.onFilterChanged(),
  );
  private readonly coincideFiltroDeColumna = (parametros: TextMatcherParams): boolean =>
    parametros.filterText === null ||
    this.decisiones.cumple(
      operacionDeFiltro(parametros.filterOption),
      parametros.filterText,
      String(parametros.value ?? ''),
    );

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
    // AG Grid pasaría el texto ya en minúsculas; el núcleo debe recibir el
    // original: la pantalla no transforma texto antes de que él decida.
    filterParams: {
      textMatcher: this.coincideFiltroDeColumna,
      textFormatter: (texto: string) => texto,
    },
    tooltipValueGetter: textoDeTooltip,
    ...(this.idFila() ? { enableCellChangeFlash: true } : {}),
  }));

  protected readonly columnasConVisibilidad: Signal<ColDef<T>[]> = computed(() =>
    this.columnas().map((columna) => {
      const clave = claveDeColumna(columna);
      return clave ? { ...columna, hide: this.ocultas().has(clave) } : columna;
    }),
  );

  /**
   * Las columnas que se pueden mostrar u ocultar, para el menú. Una columna
   * con `lockVisible` (por ejemplo, la del botón de una acción) no se ofrece:
   * siempre se ve.
   */
  protected readonly columnasDelMenu: Signal<{ clave: string; titulo: string }[]> = computed(() =>
    this.columnas().flatMap((columna) => {
      const clave = claveDeColumna(columna);
      return clave && !columna.lockVisible ? [{ clave, titulo: columna.headerName ?? clave }] : [];
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

  constructor() {
    // Lo que se escribe, las filas o las reglas cambian: se vuelve a preguntar
    // al núcleo, con una pausa para no preguntar en cada tecla.
    effect((alLimpiar) => {
      this.filtro();
      this.reglas();
      this.filas();
      this.programarBusqueda();
      alLimpiar(() => {
        clearTimeout(this.temporizadorDeBusqueda);
        this.decisiones.cancelar();
      });
    });
  }

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
    // Un filtro de columna o un orden distinto cambian qué filas se ven.
    this.programarResaltado();
  }

  private programarBusqueda(): void {
    clearTimeout(this.temporizadorDeBusqueda);
    this.temporizadorDeBusqueda = setTimeout(() => void this.buscar(), RETARDO_DE_BUSQUEDA_MS);
  }

  /** Primero qué filas pasan el buscador; después qué resaltar en las que quedan. */
  private async buscar(): Promise<void> {
    await this.filtrarFilas();
    await this.resaltarLoVisible();
  }

  private readonly claveDeFila = (fila: T): unknown => this.idFila()?.(fila) ?? fila;

  /** Las columnas que declararon su campo con `resaltable()`: las que se buscan. */
  private columnasDeBusqueda(): { campo: CampoResaltable; clave: string }[] {
    return this.columnas().flatMap((columna) => {
      const campo = campoResaltable(columna);
      const clave = claveDeColumna(columna);
      return campo && clave ? [{ campo, clave }] : [];
    });
  }

  /**
   * Le pregunta al núcleo qué filas coinciden con el buscador y filtra con su
   * respuesta (regla N2). Si el núcleo no responde, no se oculta nada.
   */
  private async filtrarFilas(): Promise<void> {
    const api = this.api;
    if (!api) {
      return;
    }
    const buscado = this.filtro().trim();
    const pedido = ++this.pedidoDeBusqueda;
    if (buscado === '') {
      this.aplicarBusqueda(api, null);
      return;
    }
    const columnas = this.columnasDeBusqueda();
    const claves: unknown[] = [];
    const filas: CampoDeTexto[][] = [];
    api.forEachNode((nodo) => {
      if (nodo.data !== undefined) {
        claves.push(this.claveDeFila(nodo.data));
        filas.push(
          columnas.map(({ campo, clave }) => ({ campo, texto: textoDeCelda(api, nodo, clave) })),
        );
      }
    });
    try {
      const respuestas = await this.reglas().filasQueCoinciden(buscado, filas);
      if (pedido === this.pedidoDeBusqueda) {
        this.aplicarBusqueda(api, new Set(claves.filter((_, i) => respuestas[i])));
      }
    } catch {
      if (pedido === this.pedidoDeBusqueda) {
        this.aplicarBusqueda(api, null);
      }
    }
  }

  private aplicarBusqueda(api: GridApi<T>, pasan: ReadonlySet<unknown> | null): void {
    if (pasan === null && this.filasQuePasan === null) {
      return;
    }
    this.filasQuePasan = pasan;
    api.onFilterChanged();
  }

  private programarResaltado(): void {
    clearTimeout(this.temporizadorDeResaltado);
    this.temporizadorDeResaltado = setTimeout(
      () => void this.resaltarLoVisible(),
      RETARDO_DE_BUSQUEDA_MS,
    );
  }

  /**
   * Pide al núcleo los segmentos de los textos que hay en pantalla, una vez
   * por columna resaltable, y repinta esas columnas. El resaltado es sólo un
   * apoyo visual: si el núcleo no responde, se muestra el texto sin resaltar.
   */
  private async resaltarLoVisible(): Promise<void> {
    const api = this.api;
    const buscado = this.filtro().trim();
    const columnas = this.columnasDeBusqueda();
    if (!api) {
      return;
    }
    const pedido = ++this.pedidoDeResaltado;
    if (buscado === '' || columnas.length === 0) {
      this.guardarResaltados(api, columnas, new Map());
      return;
    }
    const pedidos = columnas.map(({ campo, clave }) => ({
      campo,
      textos: textosDeLaColumna(api, clave),
    }));
    try {
      const respuestas = await Promise.all(
        pedidos.map(({ campo, textos }) => this.reglas().resaltar(buscado, campo, textos)),
      );
      if (pedido !== this.pedidoDeResaltado) {
        return;
      }
      const segmentos = new Map<string, readonly Segmento[]>();
      pedidos.forEach(({ campo, textos }, i) => {
        textos.forEach((texto, j) => {
          const deEsteTexto = respuestas[i]?.[j];
          if (deEsteTexto) {
            segmentos.set(claveDeTexto(campo, texto), deEsteTexto);
          }
        });
      });
      this.guardarResaltados(api, columnas, segmentos);
    } catch {
      if (pedido === this.pedidoDeResaltado) {
        this.guardarResaltados(api, columnas, new Map());
      }
    }
  }

  private guardarResaltados(
    api: GridApi<T>,
    columnas: { clave: string }[],
    segmentos: ReadonlyMap<string, readonly Segmento[]>,
  ): void {
    if (segmentos.size === 0 && this.resaltados().size === 0) {
      return;
    }
    this.resaltados.set(segmentos);
    api.refreshCells({ columns: columnas.map(({ clave }) => clave), force: true });
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

/** El texto que una celda muestra (con su formato), vacío si no tiene. */
function textoDeCelda<T>(api: GridApi<T>, nodo: IRowNode<T>, clave: string): string {
  const valor: unknown = api.getCellValue({ rowNode: nodo, colKey: clave, useFormatter: true });
  return valor === null || valor === undefined ? '' : String(valor);
}

/**
 * Los textos distintos que una columna muestra en las filas que pasan los
 * filtros, hasta [`MAXIMO_TEXTOS_POR_COLUMNA`].
 */
function textosDeLaColumna<T>(api: GridApi<T>, clave: string): string[] {
  const textos = new Set<string>();
  api.forEachNodeAfterFilter((nodo) => {
    if (textos.size >= MAXIMO_TEXTOS_POR_COLUMNA) {
      return;
    }
    const texto = textoDeCelda(api, nodo, clave);
    if (texto !== '') {
      textos.add(texto);
    }
  });
  return [...textos];
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
