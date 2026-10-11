import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  type OnInit,
  signal,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import type { CellClickedEvent, ColDef } from 'ag-grid-community';
import { Atajo } from '../compartido/atajo';
import { Avisos } from '../compartido/avisos';
import { DialogoConfirmar, type DatosConfirmar } from '../compartido/dialogo-confirmar';
import { soloFecha, soloHora } from '../compartido/formato';
import { Tabla } from '../compartido/tabla/tabla';
import { dentro, registrarSalida, registrarSalidaPorGafete } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { PersonaAdentro, Via } from '../nucleo/tipos';
import {
  etiquetaDeVia,
  filtrarPorGafete,
  textoGafete,
  textoMedio,
  textoOperador,
  type FiltroGafete,
} from './etiquetas';

/** El mensaje de un error del núcleo, o uno general si falló otra cosa. */
function mensajeDe(error: unknown, general: string): string {
  return esErrorApp(error) ? error.mensaje : general;
}

/** El título del aviso cuando falla la salida por número de gafete. */
function tituloDelError(error: unknown): string {
  if (esErrorApp(error) && error.codigo === 'gafete_sin_portador') {
    return 'Gafete no encontrado';
  }
  if (esErrorApp(error) && error.codigo === 'gafete_en_varias_vias') {
    return 'Gafete repetido';
  }
  return 'No se pudo registrar la salida';
}

/**
 * Quién está adentro ahora, por las cuatro vías. Desde aquí se registra la
 * salida (de una persona, o de varias seleccionadas); el ingreso se registra
 * en la sección de cada vía. Quien lleva demasiado tiempo adentro se resalta:
 * lo decide el núcleo.
 */
@Component({
  selector: 'app-lista-dentro',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [Atajo, MatButtonModule, MatButtonToggleModule, MatIconModule, MatTooltipModule, Tabla],
  templateUrl: './lista-dentro.html',
  styleUrl: './lista-dentro.scss',
})
export class ListaDentro implements OnInit {
  private readonly dialogos = inject(MatDialog);
  private readonly avisos = inject(Avisos);

  /** Sólo las personas de esta vía; sin ella, las cuatro juntas. */
  readonly via = input<Via | null>(null);
  /** El título de la pantalla (lo anuncian los lectores de pantalla). */
  readonly titulo = input('Dentro');
  /** Con este nombre se guarda el diseño de la grilla en este equipo. */
  readonly idTabla = input('dentro');
  readonly mensajeVacio = input('No hay nadie adentro');

  protected readonly filas = signal<PersonaAdentro[]>([]);
  protected readonly filtro = signal<FiltroGafete>('todos');
  protected readonly seleccionadas = signal<PersonaAdentro[]>([]);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);

  /** Lo que se escribe en el campo de salida por gafete: sólo dígitos. */
  protected readonly gafete = signal('');

  protected readonly visibles = computed(() => filtrarPorGafete(this.filas(), this.filtro()));

  protected readonly idFila = (fila: PersonaAdentro): string => `${fila.via}-${fila.ingreso_id}`;
  protected readonly claseFila = (fila: PersonaAdentro): string | undefined =>
    fila.demasiado_tiempo ? 'fila-demasiado-tiempo' : undefined;

  /** Las columnas según la vía: cada una muestra sólo lo que le corresponde. */
  protected readonly columnas = computed(() => this.columnasDe(this.via()));

  private columnasBase(): ColDef<PersonaAdentro>[] {
    return [
      { field: 'identidad', headerName: 'Cédula / código', flex: 1.2, minWidth: 130 },
      { field: 'nombre', headerName: 'Nombre', flex: 2, minWidth: 200 },
      {
        colId: 'via',
        headerName: 'Vía',
        flex: 1,
        minWidth: 110,
        valueGetter: (p) => (p.data ? etiquetaDeVia(p.data.via) : ''),
      },
      { field: 'procedencia', headerName: 'Empresa / motivo', flex: 1.5, minWidth: 150 },
      {
        colId: 'medio',
        headerName: 'Medio',
        flex: 1.2,
        minWidth: 130,
        valueGetter: (p) => (p.data ? textoMedio(p.data) : ''),
      },
      {
        colId: 'gafete',
        headerName: 'Gafete',
        flex: 0.8,
        minWidth: 90,
        valueGetter: (p) => (p.data ? textoGafete(p.data) : ''),
      },
      {
        colId: 'fecha',
        headerName: 'Fecha',
        flex: 1,
        minWidth: 110,
        valueGetter: (p) => (p.data ? soloFecha(p.data.entrada) : ''),
      },
      {
        colId: 'hora',
        headerName: 'Hora',
        flex: 0.7,
        minWidth: 80,
        valueGetter: (p) => (p.data ? soloHora(p.data.entrada) : ''),
      },
      {
        colId: 'dio_ingreso',
        headerName: 'Dio ingreso',
        flex: 1.3,
        minWidth: 140,
        valueGetter: (p) => (p.data ? textoOperador(p.data.entrada_por) : ''),
      },
      {
        colId: 'accion',
        headerName: '',
        width: 100,
        sortable: false,
        filter: false,
        suppressMovable: true,
        lockVisible: true,
        cellRenderer: () =>
          '<button type="button" class="boton-celda" aria-label="Registrar salida">Salida</button>',
        onCellClicked: (evento: CellClickedEvent<PersonaAdentro>) => {
          const clic = evento.event?.target as HTMLElement | null | undefined;
          if (evento.data && clic?.closest('button.boton-celda')) {
            void this.salida(evento.data);
          }
        },
      },
    ];
  }

  private columnasDe(via: Via | null): ColDef<PersonaAdentro>[] {
    return this.columnasBase().flatMap((columna): ColDef<PersonaAdentro>[] => {
      switch (columna.colId ?? columna.field) {
        case 'identidad':
          return [
            {
              ...columna,
              headerName: via === 'KOF' ? 'Código' : via === null ? 'Cédula / código' : 'Cédula',
            },
          ];
        case 'via':
          return via === null ? [columna] : [];
        case 'procedencia':
          return via === 'KOF'
            ? []
            : [
                {
                  ...columna,
                  headerName:
                    via === 'PROVEEDOR'
                      ? 'Empresa'
                      : via === 'CORREO'
                        ? 'Motivo'
                        : 'Empresa / motivo',
                },
              ];
        case 'medio':
          return via === 'KOF' ? [] : [columna];
        default:
          return [columna];
      }
    });
  }

  ngOnInit(): void {
    void this.cargar();
  }

  protected cambiarFiltro(filtro: FiltroGafete): void {
    this.filtro.set(filtro);
  }

  actualizar(): void {
    void this.cargar();
  }

  protected escribirGafete(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    campo.value = campo.value.replace(/\D/g, '');
    this.gafete.set(campo.value);
  }

  /**
   * La salida de quien devuelve este gafete, buscándolo en la vía de esta
   * lista (o en las cuatro). Quién lo tiene, o por qué no se sabe (nadie lo
   * tiene, o el número está en uso en varias vías), lo decide el núcleo.
   */
  protected async salidaPorGafete(): Promise<void> {
    const texto = this.gafete();
    if (texto === '') {
      return;
    }
    try {
      const nombre = await registrarSalidaPorGafete(this.via(), Number(texto));
      this.avisos.exito(nombre, 'Salida registrada');
      this.gafete.set('');
    } catch (error) {
      this.avisos.error(mensajeDe(error, 'inténtalo de nuevo'), tituloDelError(error));
    }
    await this.cargar();
  }

  /** La salida de una persona, sin pedir confirmación: es lo de todos los días. */
  protected async salida(persona: PersonaAdentro): Promise<void> {
    try {
      await registrarSalida(persona.via, persona.ingreso_id);
      this.avisos.exito(persona.nombre, 'Salida registrada');
    } catch (error) {
      this.avisos.error(
        `${persona.nombre}: ${mensajeDe(error, 'inténtalo de nuevo')}`,
        'No se pudo registrar la salida',
      );
    }
    await this.cargar();
  }

  /** La salida de todas las seleccionadas, después de confirmar. */
  protected salidaMasiva(): void {
    const elegidas = this.seleccionadas();
    if (elegidas.length === 0) {
      return;
    }
    const datos: DatosConfirmar = {
      titulo: 'Registrar salida',
      mensaje:
        elegidas.length === 1
          ? `¿Registrar la salida de ${elegidas[0]?.nombre}?`
          : `¿Registrar la salida de ${elegidas.length} personas?`,
      confirmar: 'Registrar salida',
    };
    this.dialogos
      .open<DialogoConfirmar, DatosConfirmar, true>(DialogoConfirmar, {
        data: datos,
        width: '380px',
      })
      .afterClosed()
      .subscribe((confirmado) => {
        if (confirmado) {
          void this.registrarVarias(elegidas);
        }
      });
  }

  private async registrarVarias(personas: PersonaAdentro[]): Promise<void> {
    const fallidas: string[] = [];
    for (const persona of personas) {
      try {
        await registrarSalida(persona.via, persona.ingreso_id);
      } catch (error) {
        fallidas.push(`${persona.nombre}: ${mensajeDe(error, 'no se pudo')}`);
      }
    }
    const hechas = personas.length - fallidas.length;
    if (fallidas.length === 0) {
      this.avisos.exito(hechas === 1 ? '1 persona' : `${hechas} personas`, 'Salidas registradas');
    } else {
      this.avisos.error(
        `Salieron ${hechas} de ${personas.length}. Fallaron: ${fallidas.join('; ')}`,
        'Salidas con problemas',
      );
    }
    this.seleccionadas.set([]);
    await this.cargar();
  }

  private async cargar(): Promise<void> {
    this.cargando.set(true);
    try {
      const via = this.via();
      const todas = await dentro();
      this.filas.set(via === null ? todas : todas.filter((p) => p.via === via));
      this.error.set(null);
    } catch (error) {
      this.error.set(mensajeDe(error, 'No se pudo cargar quién está adentro.'));
    } finally {
      this.cargando.set(false);
    }
  }
}
