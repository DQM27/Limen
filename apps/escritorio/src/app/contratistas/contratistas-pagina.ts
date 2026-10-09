import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import type { ColDef } from 'ag-grid-community';
import { fechaCorta } from '../compartido/formato';
import { Tabla } from '../compartido/tabla/tabla';
import { listarContratistas } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { FilaContratista } from '../nucleo/tipos';
import { etiquetaDeAcceso, etiquetaDeTipo } from './etiquetas';

@Component({
  selector: 'app-contratistas-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [Tabla],
  templateUrl: './contratistas-pagina.html',
  styleUrl: './contratistas-pagina.scss',
})
export class ContratistasPagina {
  protected readonly filas = signal<FilaContratista[]>([]);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);

  protected readonly idFila = (fila: FilaContratista): string => fila.id;

  protected readonly columnas: ColDef<FilaContratista>[] = [
    { field: 'cedula', headerName: 'Cédula', flex: 0, width: 140 },
    { field: 'nombre', headerName: 'Nombre', flex: 2, minWidth: 220 },
    { field: 'empresa_nombre', headerName: 'Empresa', flex: 2, minWidth: 180 },
    {
      field: 'tipo_ingreso',
      headerName: 'Tipo',
      flex: 0,
      width: 120,
      valueFormatter: (p) => (p.value ? etiquetaDeTipo(p.value) : ''),
    },
    {
      field: 'fecha_vencimiento_praind',
      headerName: 'Vence PRAIND',
      flex: 0,
      width: 150,
      valueFormatter: (p) => fechaCorta(p.value),
      // El filtro y el filtro rápido buscan lo que la persona ve, no el ISO.
      filterValueGetter: (p) => fechaCorta(p.data?.fecha_vencimiento_praind),
    },
    {
      colId: 'estado',
      headerName: 'Estado',
      flex: 1,
      minWidth: 170,
      valueGetter: (p) => (p.data ? etiquetaDeAcceso(p.data.acceso) : ''),
    },
  ];

  constructor() {
    void this.cargar();
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
