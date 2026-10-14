import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { Nucleo } from '../nucleo/nucleo';
import { ErrorPanel, type EmpresaCatalogo } from '../panel';
import { ApiContratistas } from './api-contratistas';

/**
 * Una empresa nueva, desde el formulario del contratista. La pantalla no valida nada: el nombre
 * se ve en mayúscula mientras se escribe (lo dice el núcleo) y los motivos de rechazo (vacío,
 * repetido…) llegan del núcleo por el Worker.
 */
@Component({
  selector: 'app-dialogo-empresa',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatDialogModule, MatFormFieldModule, MatInputModule],
  templateUrl: './dialogo-empresa.html',
  styleUrl: './dialogo-empresa.scss',
})
export class DialogoEmpresa {
  private readonly dialogo = inject<MatDialogRef<DialogoEmpresa, EmpresaCatalogo>>(MatDialogRef);
  private readonly nucleo = inject(Nucleo);
  private readonly api = inject(ApiContratistas);

  protected readonly nombre = signal('');
  protected readonly enviando = signal(false);
  protected readonly error = signal<string | null>(null);

  protected escribir(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const cursor = campo.selectionStart;
    const queda = this.nucleo.empresaAlEscribir(campo.value);
    if (queda !== campo.value) {
      campo.value = queda;
      campo.setSelectionRange(cursor, cursor);
    }
    this.nombre.set(queda);
    this.error.set(null);
  }

  protected async guardar(evento: Event): Promise<void> {
    evento.preventDefault();
    if (this.enviando()) {
      return;
    }
    this.enviando.set(true);
    this.error.set(null);
    try {
      this.dialogo.close(await this.api.crearEmpresa(this.nombre()));
    } catch (error) {
      this.error.set(
        error instanceof ErrorPanel && error.problemas[0]
          ? error.problemas[0].mensaje
          : 'No se pudo crear la empresa. Intente de nuevo.',
      );
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }
}
