import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import { buscarEmpresas, nombreEmpresaAlEscribir, registrarEmpresa } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { Empresa } from '../nucleo/tipos';

/**
 * El modal para dar de alta una empresa: sólo pide el nombre. Es a propósito
 * un paso aparte del formulario del contratista: crear una empresa exige ir
 * hasta aquí, así no se crea una por escribir rápido. Devuelve la empresa
 * creada; el núcleo decide qué nombre es válido y si ya existe.
 */
@Component({
  selector: 'app-dialogo-empresa',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
  ],
  templateUrl: './dialogo-empresa.html',
  styleUrl: './dialogo-empresa.scss',
})
export class DialogoEmpresa {
  private readonly dialogo = inject<MatDialogRef<DialogoEmpresa, Empresa>>(MatDialogRef);
  private readonly normalizador = new NormalizadorAlEscribir();

  readonly nombre = new FormControl('', { nonNullable: true });
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

  /** Mientras se escribe, el nombre se ve en mayúsculas, como quedará. */
  protected escribir(evento: Event): void {
    void this.normalizador.alEscribir(evento, this.nombre, nombreEmpresaAlEscribir);
  }

  protected async guardar(): Promise<void> {
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      const texto = this.nombre.value;
      const id = await registrarEmpresa(texto);
      // El nombre como quedó guardado (normalizado) lo trae el núcleo.
      const creada = (await buscarEmpresas(texto, 8).catch(() => [])).find((e) => e.id === id);
      this.dialogo.close(creada ?? { id, nombre: texto.trim() });
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }

  /** El mensaje del núcleo, junto al campo (el único que hay); si no es del núcleo, arriba de los botones. */
  private mostrar(error: unknown): void {
    if (esErrorApp(error)) {
      this.nombre.setErrors({ nucleo: error.mensaje });
      this.nombre.markAsTouched();
    } else {
      this.errorGeneral.set('No se pudo crear la empresa.');
    }
  }
}
