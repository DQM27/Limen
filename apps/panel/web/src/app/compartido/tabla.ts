import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  signal,
} from '@angular/core';
import { MatIconModule } from '@angular/material/icon';
import { AG_GRID_LOCALE_ES } from '@ag-grid-community/locale';
import {
  AllCommunityModule,
  ModuleRegistry,
  colorSchemeVariable,
  themeQuartz,
  type ColDef,
  type GetRowIdParams,
  type GridApi,
  type GridReadyEvent,
  type IRowNode,
  type ModelUpdatedEvent,
} from 'ag-grid-community';
import { AgGridAngular } from 'ag-grid-angular';
import { Nucleo, type CampoDeTexto } from '../nucleo/nucleo';

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
 * El buscador, si la pantalla da `camposDeBusqueda`, es del núcleo (regla N2): la tabla no
 * compara, no pliega ni parte por palabras; pregunta qué filas coinciden y esconde las demás.
 * Los filtros por columna de AG Grid están apagados: compararían por su cuenta. Entran cuando
 * el núcleo decida también esos filtros (regla N2, `celdas_que_cumplen`).
 */
@Component({
  selector: 'app-tabla',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [AgGridAngular, MatIconModule],
  templateUrl: './tabla.html',
  styleUrl: './tabla.scss',
})
export class Tabla<T> {
  private readonly nucleo = inject(Nucleo);

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
  protected readonly columnaPorDefecto: ColDef<T> = {
    sortable: true,
    resizable: true,
    // Los filtros de columna de AG Grid compararían por su cuenta: apagados (ver arriba).
    filter: false,
    minWidth: 90,
  };

  protected readonly buscado = signal('');
  protected readonly contexto: ContextoDeTabla = { buscado: () => this.buscado() };
  private api: GridApi<T> | null = null;
  protected readonly visibles = signal(0);

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
    const visibles = this.visibles();
    if (visibles !== total && this.coinciden() !== null) {
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
    this.buscado.set((evento.target as HTMLInputElement).value);
  }

  protected alIniciar(evento: GridReadyEvent<T>): void {
    this.api = evento.api;
    this.visibles.set(evento.api.getDisplayedRowCount());
  }

  protected alActualizarFilas(evento: ModelUpdatedEvent<T>): void {
    this.visibles.set(evento.api.getDisplayedRowCount());
  }
}
