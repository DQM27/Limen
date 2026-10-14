import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  signal,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import type { ColDef, ICellRendererParams } from 'ag-grid-community';
import { Tabla } from '../compartido/tabla';
import { DialogoConfirmar, type DatosConfirmar } from '../compartido/dialogo-confirmar';
import {
  ErrorPanel,
  listarDispositivos,
  listarEquipos,
  listarUnidades,
  retirarDispositivo,
  type Unidad,
} from '../panel';
import { DialogoEquipo, type DatosDialogoEquipo } from './dialogo-equipo';
import {
  ETIQUETA_CONEXION,
  ETIQUETA_REGISTRO,
  ETIQUETA_TIPO,
  unirEquipos,
  type FilaEquipo,
} from './filas';

const CADA_CUANTO_MS = 15_000;

const MENSAJE_ERROR: Record<string, string> = {
  nube: 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.',
};

const CLASE_REGISTRO: Record<FilaEquipo['registro'], string> = {
  esperando_codigo: 'estado-aviso',
  vinculado: 'estado-bueno',
  retirado: 'estado-malo',
  sin_registrar: 'estado-neutro',
};

const CLASE_CONEXION: Record<NonNullable<FilaEquipo['conexion']>, string> = {
  en_linea: 'estado-bueno',
  caido: 'estado-malo',
  fuera: 'estado-neutro',
};

const fecha = (valor: string | null | undefined): string =>
  valor ? new Date(valor).toLocaleString('es-CR') : '';

/**
 * Equipos: los registrados en el panel y cómo están conectados ahora, en una sola tabla.
 * No decide nada: el Worker aplica las reglas del núcleo y esta pantalla muestra lo que
 * responde.
 */
@Component({
  selector: 'app-equipos-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatIconModule, Tabla],
  templateUrl: './equipos-pagina.html',
  styleUrl: './equipos-pagina.scss',
})
export class EquiposPagina {
  private readonly dialogos = inject(MatDialog);

  protected readonly filas = signal<FilaEquipo[]>([]);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);
  private readonly unidades = signal<Unidad[]>([]);

  protected readonly idFila = (fila: FilaEquipo): string => fila.id;

  protected readonly columnas = computed<ColDef<FilaEquipo>[]>(() => [
    { field: 'nombre', headerName: 'Nombre', flex: 1.4, minWidth: 140 },
    { field: 'unidad', headerName: 'Unidad', flex: 1, minWidth: 110 },
    {
      field: 'tipo',
      headerName: 'Tipo',
      flex: 0.8,
      minWidth: 90,
      valueFormatter: ({ value }) => ETIQUETA_TIPO[value as string] ?? '',
    },
    {
      field: 'registro',
      headerName: 'Registro',
      flex: 1.2,
      minWidth: 140,
      valueFormatter: ({ value }) => ETIQUETA_REGISTRO[value as FilaEquipo['registro']],
      cellClass: ({ value }) => CLASE_REGISTRO[value as FilaEquipo['registro']],
    },
    {
      field: 'conexion',
      headerName: 'Conexión',
      flex: 1,
      minWidth: 110,
      valueFormatter: ({ value }) =>
        value ? ETIQUETA_CONEXION[value as NonNullable<FilaEquipo['conexion']>] : '—',
      cellClass: ({ value }) =>
        value ? CLASE_CONEXION[value as NonNullable<FilaEquipo['conexion']>] : 'estado-neutro',
    },
    {
      field: 'visto',
      headerName: 'Visto',
      flex: 1.4,
      minWidth: 160,
      valueFormatter: ({ value }) => fecha(value as string | null),
    },
    {
      field: 'creado',
      headerName: 'Registrado',
      flex: 1.4,
      minWidth: 160,
      valueFormatter: ({ value }) => fecha(value as string | null),
    },
    {
      colId: 'acciones',
      headerName: '',
      flex: 0.8,
      minWidth: 100,
      sortable: false,
      filter: false,
      cellRenderer: (parametros: ICellRendererParams<FilaEquipo>) => this.botonRetirar(parametros),
    },
  ]);

  constructor() {
    void this.cargar();
    const refresco = setInterval(() => {
      if (!document.hidden) {
        void this.cargar();
      }
    }, CADA_CUANTO_MS);
    inject(DestroyRef).onDestroy(() => clearInterval(refresco));
  }

  protected registrar(): void {
    const datos: DatosDialogoEquipo = { unidades: this.unidades() };
    this.dialogos
      .open(DialogoEquipo, { data: datos, autoFocus: 'dialog' })
      .afterClosed()
      .subscribe(() => void this.cargar());
  }

  /** El botón de la celda: sólo para quien todavía no está retirado. */
  private botonRetirar(parametros: ICellRendererParams<FilaEquipo>): HTMLElement | string {
    const fila = parametros.data;
    if (!fila || fila.registro === 'retirado' || fila.registro === 'sin_registrar') {
      return '';
    }
    const boton = document.createElement('button');
    boton.type = 'button';
    boton.className = 'boton-celda peligro';
    boton.textContent = 'Retirar';
    boton.addEventListener('click', () => this.retirar(fila));
    return boton;
  }

  private retirar(fila: FilaEquipo): void {
    const datos: DatosConfirmar = {
      titulo: 'Retirar equipo',
      mensaje: `¿Retirar «${fila.nombre}»? El equipo pierde el acceso al instante.`,
      confirmar: 'Retirar',
    };
    this.dialogos
      .open(DialogoConfirmar, { data: datos })
      .afterClosed()
      .subscribe((confirmado) => {
        if (confirmado) {
          void this.ejecutarRetiro(fila);
        }
      });
  }

  private async ejecutarRetiro(fila: FilaEquipo): Promise<void> {
    try {
      await retirarDispositivo(fila.id);
      await this.cargar();
    } catch (error) {
      this.error.set(this.mensaje(error));
    }
  }

  private async cargar(): Promise<void> {
    try {
      const [dispositivos, conectados, unidades] = await Promise.all([
        listarDispositivos(),
        listarEquipos(),
        listarUnidades(),
      ]);
      this.filas.set(unirEquipos(dispositivos, conectados));
      this.unidades.set(unidades);
      this.error.set(null);
    } catch (error) {
      this.error.set(this.mensaje(error));
    } finally {
      this.cargando.set(false);
    }
  }

  private mensaje(error: unknown): string {
    const codigo = error instanceof ErrorPanel ? error.codigo : '';
    return MENSAJE_ERROR[codigo] ?? 'No se pudo completar la operación. Intente de nuevo.';
  }
}
