import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import { nombrePersonaAlEscribir, registrarPersonalKof } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';

/** Lo que devuelve el modal al registrar a la persona. */
export interface ResultadoNuevoPersonal {
  nombre: string;
}

/** Sólo dígitos: lo demás que se escriba se descarta (es formato, no regla). */
function soloDigitos(texto: string): string {
  return texto.replace(/\D/g, '');
}

/**
 * El modal para dar de alta a una persona del personal KOF: su código de
 * empleado y su nombre (en mayúsculas al escribir). El núcleo decide si el
 * código vale y si ya existe; su respuesta sale junto a su campo.
 */
@Component({
  selector: 'app-dialogo-nuevo-personal',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
  ],
  templateUrl: './dialogo-nuevo-personal.html',
  styleUrl: './dialogo-nuevo-personal.scss',
})
export class DialogoNuevoPersonal {
  private readonly dialogo =
    inject<MatDialogRef<DialogoNuevoPersonal, ResultadoNuevoPersonal>>(MatDialogRef);
  private readonly normalizaNombre = new NormalizadorAlEscribir();

  readonly codigo = new FormControl('', { nonNullable: true });
  readonly nombre = new FormControl('', { nonNullable: true });
  protected readonly enviando = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);

  constructor() {
    // Un clic fuera no lo cierra (no se pierde lo escrito por accidente);
    // Escape sí, como cualquier diálogo.
    this.dialogo.keydownEvents().subscribe((evento) => {
      if (evento.key === 'Escape') {
        this.cancelar();
      }
    });
  }

  protected escribirCodigo(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const limpio = soloDigitos(campo.value);
    if (limpio !== campo.value) {
      campo.value = limpio;
    }
    this.codigo.setValue(limpio);
  }

  protected escribirNombre(evento: Event): void {
    void this.normalizaNombre.alEscribir(evento, this.nombre, nombrePersonaAlEscribir);
  }

  protected async registrar(): Promise<void> {
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      await registrarPersonalKof(this.codigo.value, this.nombre.value);
      this.dialogo.close({ nombre: this.nombre.value.trim() });
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }

  /** El mensaje del núcleo junto a su campo; sin campo, arriba de los botones. */
  private mostrar(error: unknown): void {
    if (!esErrorApp(error)) {
      this.errorGeneral.set('No se pudo registrar a la persona.');
      return;
    }
    const control =
      error.campo === 'codigo_empleado'
        ? this.codigo
        : error.campo === 'nombre'
          ? this.nombre
          : null;
    if (control) {
      control.setErrors({ nucleo: error.mensaje });
      control.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
