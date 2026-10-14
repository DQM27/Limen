import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { AG_GRID_LOCALE_ES } from '@ag-grid-community/locale';
import {
  AllCommunityModule,
  ModuleRegistry,
  colorSchemeVariable,
  themeQuartz,
  type ColDef,
  type GetRowIdParams,
} from 'ag-grid-community';
import { AgGridAngular } from 'ag-grid-angular';

// Los módulos de AG Grid se registran una sola vez, aquí.
ModuleRegistry.registerModules([AllCommunityModule]);

/**
 * La tabla de todo el panel, igual que la de la app de escritorio: AG Grid con
 * su aspecto de serie (claro u oscuro según el sistema, con los colores de
 * Tokyo Night), una sola fila de herramientas y textos en español. Cada
 * pantalla le da sus columnas y sus filas y puede poner sus propios controles a
 * la izquierda (`<... controles>`) y a la derecha (`<... acciones>`).
 *
 * Todavía sin buscador: las reglas de búsqueda son del núcleo (N1 y N2) y entran
 * con la pantalla que las necesita.
 */
@Component({
  selector: 'app-tabla',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [AgGridAngular],
  templateUrl: './tabla.html',
  styleUrl: './tabla.scss',
})
export class Tabla<T> {
  /** Cómo se llama esta tabla ("Equipos"): la anuncian los lectores de pantalla. */
  readonly etiqueta = input.required<string>();
  readonly columnas = input.required<ColDef<T>[]>();
  readonly filas = input.required<T[]>();
  readonly cargando = input(false);
  readonly mensajeVacio = input('Sin resultados');
  /** Identificador estable de cada fila: AG Grid actualiza la fila en vez de repintar todo. */
  readonly idFila = input<(fila: T) => string>();

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
    filter: true,
    minWidth: 90,
  };

  protected readonly conteo = computed(() => {
    const total = this.filas().length;
    return total === 1 ? '1 fila' : `${total} filas`;
  });

  protected readonly obtenerIdFila = computed(() => {
    const id = this.idFila();
    return id ? (parametros: GetRowIdParams<T>) => id(parametros.data) : undefined;
  });
}
