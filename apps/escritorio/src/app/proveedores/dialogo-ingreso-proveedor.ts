import { ChangeDetectionStrategy, Component, inject, signal, viewChild } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { CamposDeIngreso } from '../compartido/campos-de-ingreso';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import { SelectorEmpresa } from '../compartido/selector-empresa';
import { nombrePersonaAlEscribir, registrarEntradaProveedor } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { Empresa, Medio } from '../nucleo/tipos';

/** Lo que devuelve el modal al registrar el ingreso. */
export interface ResultadoIngresoProveedor {
  nombre: string;
}

/** El campo del núcleo (`ErrorApp.campo`) → el control del formulario que lo muestra. */
type Campo = 'cedula' | 'nombre' | 'empresa' | 'placa' | 'gafete';
const CAMPO_DEL_NUCLEO: Record<string, Campo> = {
  cedula: 'cedula',
  nombre: 'nombre',
  empresa_id: 'empresa',
  placa: 'placa',
  gafete: 'gafete',
};

/** Un campo vacío es 0: el núcleo lo rechaza como gafete inexistente. */
function aNumero(texto: string): number {
  return texto === '' ? 0 : Number.parseInt(texto, 10);
}

/**
 * El modal de ingreso de un proveedor. No hay catálogo de personas: se
 * escribe la cédula y el nombre en cada ingreso. Lleva la empresa proveedora
 * (que se elige de las que existen), cómo llegó y el número de gafete, que es
 * obligatorio. El núcleo decide qué vale y su respuesta sale junto a su campo.
 */
@Component({
  selector: 'app-dialogo-ingreso-proveedor',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    CamposDeIngreso,
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
    SelectorEmpresa,
  ],
  templateUrl: './dialogo-ingreso-proveedor.html',
  styleUrl: './dialogo-ingreso-proveedor.scss',
})
export class DialogoIngresoProveedor {
  private readonly dialogo =
    inject<MatDialogRef<DialogoIngresoProveedor, ResultadoIngresoProveedor>>(MatDialogRef);
  private readonly normalizaNombre = new NormalizadorAlEscribir();
  private readonly selector = viewChild.required(SelectorEmpresa);

  readonly cedula = new FormControl('', { nonNullable: true });
  readonly nombre = new FormControl('', { nonNullable: true });
  readonly empresa = new FormControl<string | Empresa>('', { nonNullable: true });
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
      await registrarEntradaProveedor({
        cedula: this.cedula.value,
        nombre: this.nombre.value,
        empresa_id: this.selector().idElegido(),
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
    const campo = CAMPO_DEL_NUCLEO[error.campo ?? ''];
    if (campo) {
      const control = this[campo];
      control.setErrors({ nucleo: error.mensaje });
      control.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
