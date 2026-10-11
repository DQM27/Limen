import { ChangeDetectionStrategy, Component, inject, viewChild } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { Atajo } from '../compartido/atajo';
import { Avisos } from '../compartido/avisos';
import { ListaDentro } from '../dentro/lista-dentro';
import { DialogoEntregarGafete, type ResultadoEntregaKof } from './dialogo-entregar-gafete';
import { DialogoNuevoPersonal, type ResultadoNuevoPersonal } from './dialogo-nuevo-personal';

/**
 * El personal KOF que está adentro: quien tiene un gafete provisional sin
 * devolver. Entregar el gafete es su entrada y devolverlo (la salida) se
 * registra en la lista. Desde aquí también se da de alta al personal.
 */
@Component({
  selector: 'app-kof-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [Atajo, ListaDentro, MatButtonModule, MatIconModule, MatTooltipModule],
  template: `
    <app-lista-dentro
      via="KOF"
      titulo="KOF"
      idTabla="dentro-kof"
      mensajeVacio="Nadie de KOF tiene un gafete provisional"
    >
      <button
        botones
        mat-flat-button
        type="button"
        class="entrega"
        appAtajo="F2"
        matTooltip="Entregar gafete (F2)"
        (click)="entregarGafete()"
      >
        <mat-icon aria-hidden="true">login</mat-icon>
        Entregar gafete
      </button>
      <button
        botones
        mat-stroked-button
        type="button"
        class="nuevo"
        appAtajo="Ctrl+N"
        matTooltip="Nuevo personal (Ctrl+N)"
        (click)="nuevoPersonal()"
      >
        <mat-icon aria-hidden="true">add</mat-icon>
        Nuevo personal
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
    .entrega,
    .nuevo {
      --mat-button-filled-container-height: 34px;
      --mat-button-outlined-container-height: 34px;
    }
  `,
})
export class KofPagina {
  private readonly dialogos = inject(MatDialog);
  private readonly avisos = inject(Avisos);
  private readonly lista = viewChild.required(ListaDentro);

  /** El modal que entrega el gafete provisional; al entregarlo, avisa y actualiza la lista. */
  protected entregarGafete(): void {
    this.dialogos
      .open<DialogoEntregarGafete, void, ResultadoEntregaKof>(DialogoEntregarGafete, {
        width: '420px',
        disableClose: true,
        autoFocus: 'first-tabbable',
      })
      .afterClosed()
      .subscribe((resultado) => {
        if (resultado) {
          this.avisos.exito(resultado.nombre, 'Gafete entregado');
          this.lista().actualizar();
        }
      });
  }

  /** El modal que da de alta a una persona del personal KOF. */
  protected nuevoPersonal(): void {
    this.dialogos
      .open<DialogoNuevoPersonal, void, ResultadoNuevoPersonal>(DialogoNuevoPersonal, {
        width: '380px',
        disableClose: true,
        autoFocus: 'first-tabbable',
      })
      .afterClosed()
      .subscribe((resultado) => {
        if (resultado) {
          this.avisos.exito(resultado.nombre, 'Personal registrado');
        }
      });
  }
}
