import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { CamposDeIngreso } from '../compartido/campos-de-ingreso';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import { nombrePersonaAlEscribir, registrarEntradaCorreo } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { Medio } from '../nucleo/tipos';

/** Lo que devuelve el modal al registrar el ingreso. */
export interface ResultadoIngresoCorreo {
  nombre: string;
}

/** El campo del núcleo (`ErrorApp.campo`) → el control del formulario que lo muestra. */
type Campo = 'cedula' | 'nombre' | 'motivo' | 'placa' | 'gafete';
const CAMPOS_DEL_NUCLEO: readonly string[] = ['cedula', 'nombre', 'motivo', 'placa', 'gafete'];

/** Un campo vacío es 0: el núcleo lo rechaza como gafete inexistente. */
function aNumero(texto: string): number {
  return texto === '' ? 0 : Number.parseInt(texto, 10);
}

/**
 * El modal de ingreso por correo: una visita autorizada por correo. No hay
 * catálogo de personas: se escribe la cédula y el nombre en cada ingreso, más
 * el motivo (a quién visita), cómo llegó y el gafete de visita, obligatorio.
 * El núcleo decide qué vale y su respuesta sale junto a su campo.
 */
@Component({
  selector: 'app-dialogo-ingreso-correo',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    CamposDeIngreso,
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
  ],
  templateUrl: './dialogo-ingreso-correo.html',
  styleUrl: './dialogo-ingreso-correo.scss',
})
export class DialogoIngresoCorreo {
  private readonly dialogo =
    inject<MatDialogRef<DialogoIngresoCorreo, ResultadoIngresoCorreo>>(MatDialogRef);
  private readonly normalizaNombre = new NormalizadorAlEscribir();

  readonly cedula = new FormControl('', { nonNullable: true });
  readonly nombre = new FormControl('', { nonNullable: true });
  readonly motivo = new FormControl('', { nonNullable: true });
  readonly medio = new FormControl<Medio>('A_PIE', { nonNullable: true });
  readonly placa = new FormControl('', { nonNullable: true });
  readonly gafete = new FormControl('', { nonNullable: true });

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
      await registrarEntradaCorreo({
        cedula: this.cedula.value,
        nombre: this.nombre.value,
        motivo: this.motivo.value,
        medio: this.medio.value,
        placa: this.medio.value === 'VEHICULO' ? this.placa.value : null,
        gafete: aNumero(this.gafete.value),
      });
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
      this.errorGeneral.set('No se pudo registrar el ingreso.');
      return;
    }
    if (error.campo && CAMPOS_DEL_NUCLEO.includes(error.campo)) {
      const control = this[error.campo as Campo];
      control.setErrors({ nucleo: error.mensaje });
      control.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
