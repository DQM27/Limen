import { ChangeDetectionStrategy, Component, inject, viewChild } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { Atajo } from '../compartido/atajo';
import { Avisos } from '../compartido/avisos';
import { ListaDentro } from '../dentro/lista-dentro';
import { DialogoIngresoCorreo, type ResultadoIngresoCorreo } from './dialogo-ingreso-correo';

/**
 * Las visitas por correo que están adentro, con su salida y el botón para
 * registrar un ingreso. No hay catálogo de personas: cada ingreso lleva sus
 * datos y su motivo.
 */
@Component({
  selector: 'app-correo-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [Atajo, ListaDentro, MatButtonModule, MatIconModule, MatTooltipModule],
  template: `
    <app-lista-dentro
      via="CORREO"
      titulo="Correo"
      idTabla="dentro-correo"
      mensajeVacio="No hay visitas por correo adentro"
    >
      <button
        botones
        mat-flat-button
        type="button"
        class="ingreso"
        appAtajo="F2"
        matTooltip="Registrar ingreso (F2)"
        (click)="registrarIngreso()"
      >
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
export class CorreoPagina {
  private readonly dialogos = inject(MatDialog);
  private readonly avisos = inject(Avisos);
  private readonly lista = viewChild.required(ListaDentro);

  /** El modal de ingreso; al registrar, avisa y actualiza la lista. */
  protected registrarIngreso(): void {
    this.dialogos
      .open<DialogoIngresoCorreo, void, ResultadoIngresoCorreo>(DialogoIngresoCorreo, {
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
