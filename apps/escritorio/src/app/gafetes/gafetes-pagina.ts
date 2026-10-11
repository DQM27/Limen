import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { MatSnackBar } from '@angular/material/snack-bar';
import type { ColDef } from 'ag-grid-community';
import { Tabla } from '../compartido/tabla/tabla';
import { listarGafetes } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { Gafete, TipoGafete } from '../nucleo/tipos';
import { DialogoCrearGafetes, type DatosCrearGafetes } from './dialogo-crear-gafetes';
import {
  DialogoGestionGafete,
  type DatosGestionGafete,
  type ResultadoGestionGafete,
} from './dialogo-gestion-gafete';
import { etiquetaDeEstadoGafete, TIPOS_DE_GAFETE, textoAsignado } from './etiquetas';

/** Lo que se avisa al operador tras cada cambio de estado. */
function avisoDeCambio(numero: number, cambio: ResultadoGestionGafete['cambio']): string {
  switch (cambio) {
    case 'PERDIDO':
      return `Gafete ${numero} marcado como perdido`;
    case 'PAGADO':
      return `Gafete ${numero} pagado: vuelve a estar disponible`;
    case 'APARECIDO':
      return `Gafete ${numero} apareció: vuelve a estar disponible`;
    case 'DE_BAJA':
      return `Gafete ${numero} dado de baja`;
  }
}

/**
 * El catálogo de gafetes, de un tipo a la vez (un número se repite entre
 * tipos, así que no se mezclan). Doble clic en uno lo gestiona; "Nuevos
 * gafetes" los da de alta por rango. Qué se puede hacer con cada uno lo
 * decide el núcleo.
 */
@Component({
  selector: 'app-gafetes-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatButtonToggleModule, MatIconModule, Tabla],
  templateUrl: './gafetes-pagina.html',
  styleUrl: './gafetes-pagina.scss',
})
export class GafetesPagina {
  private readonly dialogos = inject(MatDialog);
  private readonly aviso = inject(MatSnackBar);

  protected readonly tipos = TIPOS_DE_GAFETE;
  protected readonly tipo = signal<TipoGafete>('CONTRATISTA');
  protected readonly filas = signal<Gafete[]>([]);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);

  protected readonly idFila = (fila: Gafete): string => `${fila.tipo}-${fila.numero}`;

  protected readonly columnas: ColDef<Gafete>[] = [
    { field: 'numero', type: 'numero', headerName: 'Número', flex: 1, minWidth: 100 },
    {
      colId: 'estado',
      headerName: 'Estado',
      flex: 1,
      minWidth: 120,
      valueGetter: (p) => (p.data ? etiquetaDeEstadoGafete(p.data) : ''),
    },
    {
      colId: 'asignado',
      headerName: 'Asignado a',
      flex: 3,
      minWidth: 220,
      valueGetter: (p) => (p.data ? textoAsignado(p.data) : ''),
    },
  ];

  constructor() {
    void this.cargar();
  }

  protected cambiarTipo(tipo: TipoGafete): void {
    this.tipo.set(tipo);
    void this.cargar();
  }

  /** "Nuevos gafetes": el modal del rango; al crear, se recarga y se avisa cuántos. */
  protected crear(): void {
    const datos: DatosCrearGafetes = { tipo: this.tipo() };
    this.dialogos
      .open<DialogoCrearGafetes, DatosCrearGafetes, number>(DialogoCrearGafetes, {
        data: datos,
        width: '340px',
        disableClose: true,
        autoFocus: 'first-tabbable',
      })
      .afterClosed()
      .subscribe((creados) => {
        if (creados !== undefined) {
          void this.cargar();
          this.aviso.open(
            creados === 1 ? 'Se creó 1 gafete' : `Se crearon ${creados} gafetes`,
            'Cerrar',
            { duration: 4000 },
          );
        }
      });
  }

  /** Doble clic en un gafete: el modal con lo que se puede hacer con él. */
  protected gestionar(gafete: Gafete): void {
    const datos: DatosGestionGafete = { gafete };
    this.dialogos
      .open<DialogoGestionGafete, DatosGestionGafete, ResultadoGestionGafete>(
        DialogoGestionGafete,
        { data: datos, width: '380px', disableClose: true, autoFocus: 'dialog' },
      )
      .afterClosed()
      .subscribe((resultado) => {
        if (resultado) {
          void this.cargar();
          this.aviso.open(avisoDeCambio(gafete.numero, resultado.cambio), 'Cerrar', {
            duration: 4000,
          });
        }
      });
  }

  private async cargar(): Promise<void> {
    this.cargando.set(true);
    try {
      this.filas.set(await listarGafetes(this.tipo()));
      this.error.set(null);
    } catch (error) {
      this.error.set(esErrorApp(error) ? error.mensaje : 'No se pudieron cargar los gafetes.');
    } finally {
      this.cargando.set(false);
    }
  }
}
