import { ChangeDetectionStrategy, Component, inject, viewChild } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { Avisos } from '../compartido/avisos';
import { ListaDentro } from '../dentro/lista-dentro';
import {
  DialogoIngresoProveedor,
  type ResultadoIngresoProveedor,
} from './dialogo-ingreso-proveedor';

/**
 * Los proveedores que están adentro, con su salida y el botón para registrar
 * un ingreso. No hay catálogo de personas: cada ingreso lleva sus datos.
 */
@Component({
  selector: 'app-proveedores-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ListaDentro, MatButtonModule, MatIconModule],
  template: `
    <app-lista-dentro
      via="PROVEEDOR"
      titulo="Proveedores"
      idTabla="dentro-proveedores"
      mensajeVacio="No hay proveedores adentro"
    >
      <button botones mat-flat-button type="button" class="ingreso" (click)="registrarIngreso()">
        <mat-icon aria-hidden="true">login</mat-icon>
        Registrar ingreso
      </button>
    </app-lista-dentro>
  `,
  styles: `
    :host {
      display: flex;
      flex: 1;
      flex-direction: column;
      min-height: 0;
    }
    .ingreso {
      --mat-button-filled-container-height: 34px;
    }
  `,
})
export class ProveedoresPagina {
  private readonly dialogos = inject(MatDialog);
  private readonly avisos = inject(Avisos);
  private readonly lista = viewChild.required(ListaDentro);

  /** El modal de ingreso; al registrar, avisa y actualiza la lista. */
  protected registrarIngreso(): void {
    this.dialogos
      .open<DialogoIngresoProveedor, void, ResultadoIngresoProveedor>(DialogoIngresoProveedor, {
        width: '420px',
        disableClose: true,
        autoFocus: 'first-tabbable',
      })
      .afterClosed()
      .subscribe((resultado) => {
        if (resultado) {
          this.avisos.exito(resultado.nombre, 'Ingreso registrado');
          this.lista().actualizar();
        }
      });
  }
}
