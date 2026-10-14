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
import { Avisos } from '../compartido/avisos';
import { fechaHora } from '../compartido/fecha';
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
  EXPLICACION_CONEXION,
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
  private readonly avisos = inject(Avisos);
  /** El último error que se avisó: si sigue igual (el refresco cada 15 s) no se repite. */
  private ultimoError: string | null = null;
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
      minWidth: 120,
      valueFormatter: ({ value }) =>
        value ? ETIQUETA_CONEXION[value as NonNullable<FilaEquipo['conexion']>] : '—',
      cellClass: ({ value }) =>
        value ? CLASE_CONEXION[value as NonNullable<FilaEquipo['conexion']>] : 'estado-neutro',
      // Qué significa, al pasar el cursor.
      tooltipValueGetter: ({ value }) =>
        value
          ? EXPLICACION_CONEXION[value as NonNullable<FilaEquipo['conexion']>]
          : 'Este equipo todavía no ha abierto la app.',
    },
    {
      field: 'iniciada',
      headerName: 'Conectado desde',
      flex: 1.4,
      minWidth: 160,
      valueFormatter: ({ value }) => fechaHora(value as string | null),
    },
    {
      field: 'visto',
      headerName: 'Última señal',
      flex: 1.4,
      minWidth: 160,
      valueFormatter: ({ value }) => fechaHora(value as string | null),
    },
    {
      field: 'creado',
      headerName: 'Registrado',
      flex: 1.4,
      minWidth: 160,
      valueFormatter: ({ value }) => fechaHora(value as string | null),
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
      this.avisos.exito(`«${fila.nombre}» ya no tiene acceso a la nube.`, 'Equipo retirado');
      await this.cargar();
    } catch (error) {
      this.fallo(this.mensaje(error));
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
      this.ultimoError = null;
    } catch (error) {
      this.fallo(this.mensaje(error));
    } finally {
      this.cargando.set(false);
    }
  }

  private fallo(mensaje: string): void {
    if (mensaje !== this.ultimoError) {
      this.avisos.error(mensaje);
    }
    this.ultimoError = mensaje;
  }

  private mensaje(error: unknown): string {
    const codigo = error instanceof ErrorPanel ? error.codigo : '';
    return MENSAJE_ERROR[codigo] ?? 'No se pudo completar la operación. Intente de nuevo.';
  }
}
