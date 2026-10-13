import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';

export interface DatosConfirmar {
  titulo: string;
  mensaje: string;
  /** El texto del botón que confirma ("Registrar salida"). */
  confirmar: string;
}

/**
 * Una confirmación sencilla: un título, un mensaje y dos botones. Se cierra
 * con `true` si se confirma; con cualquier otra salida (Cancelar, Escape) no
 * devuelve nada.
 */
@Component({
  selector: 'app-dialogo-confirmar',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatDialogModule],
  template: `
    <h2 mat-dialog-title>{{ datos.titulo }}</h2>
    <mat-dialog-content>
      <p class="mensaje">{{ datos.mensaje }}</p>
    </mat-dialog-content>
    <mat-dialog-actions align="end">
      <button mat-button type="button" (click)="dialogo.close()">Cancelar</button>
      <button mat-flat-button type="button" (click)="dialogo.close(true)">
        {{ datos.confirmar }}
      </button>
    </mat-dialog-actions>
  `,
  styles: `
    h2 {
      font: var(--mat-sys-title-large);
    }
    .mensaje {
      margin: 0;
    }
  `,
})
export class DialogoConfirmar {
  protected readonly dialogo = inject<MatDialogRef<DialogoConfirmar, true>>(MatDialogRef);
  protected readonly datos = inject<DatosConfirmar>(MAT_DIALOG_DATA);
}
