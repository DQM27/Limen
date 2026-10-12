import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import type { ColDef } from 'ag-grid-community';
import { fechaCorta } from '../compartido/formato';
import { resaltable } from '../compartido/tabla/resaltado';
import { Tabla } from '../compartido/tabla/tabla';
import { reglasDelNucleo } from '../nucleo/busqueda';
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
  protected readonly reglas = reglasDelNucleo;

  protected readonly columnas: ColDef<FilaContratista>[] = [
    resaltable(
      { field: 'cedula', headerName: 'Cédula', flex: 1.2, minWidth: 120 },
      'IDENTIFICACION',
    ),
    resaltable({ field: 'nombre', headerName: 'Nombre', flex: 2, minWidth: 220 }, 'NOMBRE'),
    resaltable(
      { field: 'empresa_nombre', headerName: 'Empresa', flex: 1.5, minWidth: 150 },
      'EMPRESA',
    ),
    {
      field: 'tipo_ingreso',
      headerName: 'Tipo',
      flex: 1,
      minWidth: 100,
      valueGetter: (p) => (p.data ? etiquetaDeTipo(p.data.tipo_ingreso) : ''),
    },
    {
      field: 'fecha_vencimiento_praind',
      type: 'fecha',
      headerName: 'Vence PRAIND',
      flex: 1.2,
      minWidth: 140,
      valueFormatter: (p) => fechaCorta(p.value),
    },
    {
      colId: 'estado',
      headerName: 'Estado',
      flex: 1.2,
      minWidth: 150,
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
