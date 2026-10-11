import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import type { CellClickedEvent, ColDef } from 'ag-grid-community';
import { Avisos } from '../compartido/avisos';
import { DialogoConfirmar, type DatosConfirmar } from '../compartido/dialogo-confirmar';
import { soloFecha, soloHora } from '../compartido/formato';
import { Tabla } from '../compartido/tabla/tabla';
import { dentro, registrarSalida } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { PersonaAdentro } from '../nucleo/tipos';
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

/**
 * Quién está adentro ahora, por las cuatro vías. Desde aquí se registra la
 * salida (de una persona, o de varias seleccionadas); el ingreso se registra
 * en la sección de cada vía. Quien lleva demasiado tiempo adentro se resalta:
 * lo decide el núcleo.
 */
@Component({
  selector: 'app-dentro-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatButtonToggleModule, MatIconModule, MatTooltipModule, Tabla],
  templateUrl: './dentro-pagina.html',
  styleUrl: './dentro-pagina.scss',
})
export class DentroPagina {
  private readonly dialogos = inject(MatDialog);
  private readonly avisos = inject(Avisos);

  protected readonly filas = signal<PersonaAdentro[]>([]);
  protected readonly filtro = signal<FiltroGafete>('todos');
  protected readonly seleccionadas = signal<PersonaAdentro[]>([]);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);

  protected readonly visibles = computed(() => filtrarPorGafete(this.filas(), this.filtro()));

  protected readonly idFila = (fila: PersonaAdentro): string => `${fila.via}-${fila.ingreso_id}`;
  protected readonly claseFila = (fila: PersonaAdentro): string | undefined =>
    fila.demasiado_tiempo ? 'fila-demasiado-tiempo' : undefined;

  protected readonly columnas: ColDef<PersonaAdentro>[] = [
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
      cellRenderer: () =>
        '<button type="button" class="boton-salida" aria-label="Registrar salida">Salida</button>',
      onCellClicked: (evento: CellClickedEvent<PersonaAdentro>) => {
        const clic = evento.event?.target as HTMLElement | null | undefined;
        if (evento.data && clic?.closest('button.boton-salida')) {
          void this.salida(evento.data);
        }
      },
    },
  ];

  constructor() {
    void this.cargar();
  }

  protected cambiarFiltro(filtro: FiltroGafete): void {
    this.filtro.set(filtro);
  }

  protected actualizar(): void {
    void this.cargar();
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
      this.filas.set(await dentro());
      this.error.set(null);
    } catch (error) {
      this.error.set(mensajeDe(error, 'No se pudo cargar quién está adentro.'));
    } finally {
      this.cargando.set(false);
    }
  }
}
