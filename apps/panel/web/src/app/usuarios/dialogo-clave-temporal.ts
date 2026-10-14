import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';

export interface DatosClaveTemporal {
  cedula: string;
  nombre: string;
  /** La clave temporal que acaba de dar el Worker: se ve esta vez y no se vuelve a pedir. */
  clave: string;
}

/**
 * La clave temporal de un usuario recién creado o a quien se le restableció la clave. Se
 * muestra una sola vez para copiarla y enviársela a la persona (WhatsApp, en persona…); no
 * queda guardada en ningún lado y la persona la cambia al entrar.
 */
@Component({
  selector: 'app-dialogo-clave-temporal',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatDialogModule, MatIconModule],
  templateUrl: './dialogo-clave-temporal.html',
  styleUrl: './dialogo-clave-temporal.scss',
})
export class DialogoClaveTemporal {
  private readonly dialogo = inject<MatDialogRef<DialogoClaveTemporal>>(MatDialogRef);
  protected readonly datos = inject<DatosClaveTemporal>(MAT_DIALOG_DATA);
  protected readonly copiada = signal(false);

  protected async copiar(): Promise<void> {
    try {
      await navigator.clipboard.writeText(this.datos.clave);
      this.copiada.set(true);
    } catch {
      // Sin permiso para el portapapeles, la clave sigue a la vista para escribirla o seleccionarla.
      this.copiada.set(false);
    }
  }

  protected cerrar(): void {
    this.dialogo.close();
  }
}
