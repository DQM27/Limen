import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  inject,
  signal,
} from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { enfocarPrimerError } from '../compartido/enfocar-error';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import {
  nombrePersonaAlEscribir,
  registrarPersonalKof,
  validarPersonalKof,
} from '../nucleo/comandos';
import { esErrorApp, type ErrorApp } from '../nucleo/tauri';

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
  private readonly anfitrion = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly injector = inject(Injector);
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
    this.limpiarErrores();
    this.enviando.set(true);
    try {
      // Primero se pregunta qué falta o qué está mal, todo de una vez: cada
      // problema se marca en su campo (lo decide el dominio).
      const problemas = await validarPersonalKof(this.codigo.value, this.nombre.value);
      if (problemas.length > 0) {
        this.mostrarTodos(problemas);
        return;
      }
      await registrarPersonalKof(this.codigo.value, this.nombre.value);
      this.dialogo.close({ nombre: this.nombre.value.trim() });
    } catch (error) {
      this.mostrarTodos([
        esErrorApp(error)
          ? error
          : {
              tipo: 'tecnico',
              codigo: 'desconocido',
              mensaje: 'No se pudo registrar a la persona.',
            },
      ]);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }

  private limpiarErrores(): void {
    this.errorGeneral.set(null);
    this.codigo.setErrors(null);
    this.nombre.setErrors(null);
  }

  /** El mensaje del núcleo junto a su campo; sin campo, arriba de los botones. */
  private mostrarTodos(problemas: ErrorApp[]): void {
    const generales: string[] = [];
    for (const problema of problemas) {
      const control =
        problema.campo === 'codigo_empleado'
          ? this.codigo
          : problema.campo === 'nombre'
            ? this.nombre
            : null;
      if (!control) {
        generales.push(problema.mensaje);
      } else if (!control.errors) {
        control.setErrors({ nucleo: problema.mensaje });
        control.markAsTouched();
      }
    }
    this.errorGeneral.set(generales.length > 0 ? generales.join(' · ') : null);
    enfocarPrimerError(this.anfitrion, this.injector);
  }
}
