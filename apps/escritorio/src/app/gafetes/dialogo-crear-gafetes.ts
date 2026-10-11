import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { registrarGafetes } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { TipoGafete } from '../nucleo/tipos';
import { etiquetaDeTipoGafete } from './etiquetas';

export interface DatosCrearGafetes {
  tipo: TipoGafete;
}

/** Sólo dígitos: lo demás que se escriba se descarta (es formato, no regla). */
function soloDigitos(texto: string): string {
  return texto.replace(/\D/g, '');
}

/** Un campo vacío es 0: el núcleo lo rechaza como rango inválido. */
function aNumero(texto: string): number {
  return texto === '' ? 0 : Number.parseInt(texto, 10);
}

/**
 * El modal para dar de alta gafetes de un tipo, por rango (de uno a otro).
 * Para uno solo, "Hasta" se deja vacío. Devuelve cuántos se crearon; el núcleo
 * decide si el rango es válido y si algún número ya existe.
 */
@Component({
  selector: 'app-dialogo-crear-gafetes',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
  ],
  templateUrl: './dialogo-crear-gafetes.html',
  styleUrl: './dialogo-crear-gafetes.scss',
})
export class DialogoCrearGafetes {
  private readonly dialogo = inject<MatDialogRef<DialogoCrearGafetes, number>>(MatDialogRef);
  private readonly datos = inject<DatosCrearGafetes>(MAT_DIALOG_DATA);

  protected readonly etiquetaTipo = etiquetaDeTipoGafete(this.datos.tipo);
  readonly desde = new FormControl('', { nonNullable: true });
  readonly hasta = new FormControl('', { nonNullable: true });
  protected readonly enviando = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);

  constructor() {
    // Un clic fuera no lo cierra; Escape sí.
    this.dialogo.keydownEvents().subscribe((evento) => {
      if (evento.key === 'Escape') {
        this.cancelar();
      }
    });
  }

  protected escribir(control: FormControl<string>, evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const limpio = soloDigitos(campo.value);
    if (limpio !== campo.value) {
      campo.value = limpio;
    }
    control.setValue(limpio);
  }

  protected async guardar(): Promise<void> {
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      const desde = aNumero(this.desde.value);
      const hasta = this.hasta.value === '' ? desde : aNumero(this.hasta.value);
      const creados = await registrarGafetes(this.datos.tipo, desde, hasta);
      this.dialogo.close(creados);
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }

  /** Un error del rango va junto a "Hasta"; los demás, arriba de los botones. */
  private mostrar(error: unknown): void {
    if (!esErrorApp(error)) {
      this.errorGeneral.set('No se pudieron crear los gafetes.');
      return;
    }
    if (error.campo === 'hasta') {
      this.hasta.setErrors({ nucleo: error.mensaje });
      this.hasta.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
