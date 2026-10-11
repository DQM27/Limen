import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import type { CellClickedEvent, ColDef } from 'ag-grid-community';
import { Atajo } from '../compartido/atajo';
import { Avisos } from '../compartido/avisos';
import { fechaCorta } from '../compartido/formato';
import { Tabla } from '../compartido/tabla/tabla';
import { listarContratistas } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { FilaContratista } from '../nucleo/tipos';
import { DialogoIngreso, type DatosIngreso, type ResultadoIngreso } from './dialogo-ingreso';
import { etiquetaDeAcceso, etiquetaDeTipo } from './etiquetas';
import {
  FormularioContratista,
  type DatosFormularioContratista,
  type ResultadoFormularioContratista,
} from './formulario-contratista';
import { CambiosDeLaNube } from '../nucleo/cambios-de-la-nube';

/** El buscador general sólo mira lo que identifica a la persona. */
const fueraDelBuscador = (): string => '';

@Component({
  selector: 'app-contratistas-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [Atajo, MatButtonModule, MatIconModule, MatTooltipModule, Tabla],
  templateUrl: './contratistas-pagina.html',
  styleUrl: './contratistas-pagina.scss',
})
export class ContratistasPagina {
  private readonly dialogo = inject(MatDialog);
  private readonly avisos = inject(Avisos);

  protected readonly filas = signal<FilaContratista[]>([]);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);

  protected readonly idFila = (fila: FilaContratista): string => fila.id;

  protected readonly columnas: ColDef<FilaContratista>[] = [
    { field: 'cedula', headerName: 'Cédula', flex: 1.2, minWidth: 120 },
    { field: 'nombre', headerName: 'Nombre', flex: 2, minWidth: 220 },
    { field: 'empresa_nombre', headerName: 'Empresa', flex: 1.5, minWidth: 150 },
    {
      field: 'tipo_ingreso',
      headerName: 'Tipo',
      flex: 1,
      minWidth: 100,
      valueGetter: (p) => (p.data ? etiquetaDeTipo(p.data.tipo_ingreso) : ''),
      getQuickFilterText: fueraDelBuscador,
    },
    {
      field: 'fecha_vencimiento_praind',
      type: 'fecha',
      headerName: 'Vence PRAIND',
      flex: 1.2,
      minWidth: 140,
      valueFormatter: (p) => fechaCorta(p.value),
      getQuickFilterText: fueraDelBuscador,
    },
    {
      colId: 'estado',
      headerName: 'Estado',
      flex: 1.2,
      minWidth: 150,
      valueGetter: (p) => (p.data ? etiquetaDeAcceso(p.data.acceso) : ''),
      getQuickFilterText: fueraDelBuscador,
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
        '<button type="button" class="boton-celda" aria-label="Registrar ingreso">Ingreso</button>',
      onCellClicked: (evento: CellClickedEvent<FilaContratista>) => {
        const clic = evento.event?.target as HTMLElement | null | undefined;
        if (evento.data && clic?.closest('button.boton-celda')) {
          this.registrarIngreso(evento.data.id);
        }
      },
    },
  ];

  constructor() {
    void this.cargar();
    // Lo que registran los demás equipos llega de la nube: se vuelve a cargar.
    inject(CambiosDeLaNube).alLlegar(() => void this.cargar());
  }

  /**
   * El ingreso: con el id de un contratista abre directo en su ficha; sin él,
   * empieza por buscarlo. Lo que se puede o no lo decide el núcleo.
   */
  protected registrarIngreso(contratistaId?: string): void {
    const datos: DatosIngreso = { contratistaId };
    this.dialogo
      .open<DialogoIngreso, DatosIngreso, ResultadoIngreso>(DialogoIngreso, {
        data: datos,
        width: '460px',
        disableClose: true,
        autoFocus: 'first-tabbable',
      })
      .afterClosed()
      .subscribe((resultado) => {
        if (!resultado) {
          return;
        }
        this.avisos.exito(resultado.nombre, 'Ingreso registrado');
        if (resultado.acceso.resultado === 'PERMITIDO_CON_ADVERTENCIA') {
          this.avisos.info(etiquetaDeAcceso(resultado.acceso), 'PRAIND por vencer');
        }
      });
  }

  /** Abre el formulario: sin contratista registra uno nuevo; con él, lo edita. */
  protected abrirFormulario(contratista?: FilaContratista): void {
    const datos: DatosFormularioContratista = { contratista };
    this.dialogo
      .open<FormularioContratista, DatosFormularioContratista, ResultadoFormularioContratista>(
        FormularioContratista,
        { data: datos, autoFocus: 'first-tabbable', width: '380px', disableClose: true },
      )
      .afterClosed()
      .subscribe((resultado) => {
        if (!resultado) {
          return;
        }
        void this.cargar();
        if (resultado.tipo === 'editado' && resultado.cambios.length === 0) {
          this.avisos.info('Sin cambios');
        } else {
          this.avisos.exito(
            resultado.tipo === 'registrado' ? 'Contratista registrado' : 'Contratista actualizado',
          );
        }
      });
  }

  private async cargar(): Promise<void> {
    try {
      this.filas.set(await listarContratistas());
    } catch (error) {
      this.error.set(esErrorApp(error) ? error.mensaje : 'No se pudieron cargar los contratistas.');
    } finally {
      this.cargando.set(false);
    }
  }
}
