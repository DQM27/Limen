import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  signal,
  type Signal,
} from '@angular/core';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { AG_GRID_LOCALE_ES } from '@ag-grid-community/locale';
import {
  AllCommunityModule,
  ModuleRegistry,
  colorSchemeVariable,
  themeQuartz,
  type ColDef,
  type GetRowIdParams,
  type ModelUpdatedEvent,
} from 'ag-grid-community';
import { AgGridAngular } from 'ag-grid-angular';

// Los módulos de AG Grid se registran una sola vez, aquí. Hoy todos los de la
// edición Community; si el tamaño del paquete llegara a importar, se acota en
// este único lugar.
ModuleRegistry.registerModules([AllCommunityModule]);

/**
 * La tabla de toda la app. Cada pantalla le da sus columnas y sus filas, y
 * hereda el diseño unificado: aspecto de serie de AG Grid (claro u oscuro según
 * el sistema), textos en español, filtro rápido, filtro y orden por columna,
 * paginación, conteo de registros, estados de carga y vacío, y su nombre para
 * lectores de pantalla. Un cambio de diseño se hace aquí y lo heredan todas.
 */
@Component({
  selector: 'app-tabla',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [AgGridAngular, MatFormFieldModule, MatIconModule, MatInputModule],
  templateUrl: './tabla.html',
  styleUrl: './tabla.scss',
})
export class Tabla<T> {
  /** Cómo se llama esta tabla ("Contratistas"): la anuncian los lectores de pantalla. */
  readonly etiqueta = input.required<string>();
  readonly columnas = input.required<ColDef<T>[]>();
  readonly filas = input.required<T[]>();
  readonly cargando = input(false);
  readonly mensajeVacio = input('No hay registros para mostrar');
  /** Identificador estable de cada fila; mantiene la selección al refrescar. */
  readonly idFila = input<(fila: T) => string>();

  protected readonly textos = AG_GRID_LOCALE_ES;
  /** Quartz de serie; el esquema (claro u oscuro) sigue al del sistema. */
  protected readonly tema = themeQuartz.withPart(colorSchemeVariable);
  protected readonly columnaPorDefecto: ColDef<T> = {
    sortable: true,
    filter: true,
    resizable: true,
    minWidth: 100,
    flex: 1,
  };

  protected readonly filtro = signal('');
  /** Cuántas filas se ven con el filtro puesto. */
  protected readonly visibles = signal<number | null>(null);

  protected readonly total: Signal<number> = computed(() => this.filas().length);
  protected readonly conteo: Signal<string> = computed(() => {
    const total = this.total();
    const visibles = this.visibles() ?? total;
    const palabra = (n: number): string => (n === 1 ? 'registro' : 'registros');
    return visibles === total
      ? `${total} ${palabra(total)}`
      : `${visibles} de ${total} ${palabra(total)}`;
  });

  protected readonly obtenerIdFila = computed(() => {
    const id = this.idFila();
    return id ? (parametros: GetRowIdParams<T>): string => id(parametros.data) : undefined;
  });

  constructor() {
    ajustarEsquemaDelGrid();
  }

  protected escribirFiltro(evento: Event): void {
    this.filtro.set((evento.target as HTMLInputElement).value);
  }

  protected alActualizarFilas(evento: ModelUpdatedEvent<T>): void {
    this.visibles.set(evento.api.getDisplayedRowCount());
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
