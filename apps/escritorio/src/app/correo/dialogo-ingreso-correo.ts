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
import { CamposDeIngreso } from '../compartido/campos-de-ingreso';
import { enfocarPrimerError } from '../compartido/enfocar-error';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import {
  nombrePersonaAlEscribir,
  registrarEntradaCorreo,
  validarEntradaCorreo,
} from '../nucleo/comandos';
import { esErrorApp, type ErrorApp } from '../nucleo/tauri';
import type { Medio } from '../nucleo/tipos';

/** Lo que devuelve el modal al registrar el ingreso. */
export interface ResultadoIngresoCorreo {
  nombre: string;
}

/** El campo del núcleo (`ErrorApp.campo`) → el control del formulario que lo muestra. */
const CAMPOS_DEL_NUCLEO = ['cedula', 'nombre', 'motivo', 'placa', 'gafete'] as const;

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
  private readonly anfitrion = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly injector = inject(Injector);
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
    this.limpiarErrores();
    this.enviando.set(true);
    try {
      const entrada = {
        cedula: this.cedula.value,
        nombre: this.nombre.value,
        motivo: this.motivo.value,
        medio: this.medio.value,
        placa: this.medio.value === 'VEHICULO' ? this.placa.value : null,
        gafete: aNumero(this.gafete.value),
      };
      // Primero se pregunta qué falta o qué está mal, todo de una vez: cada
      // problema se marca en su campo (lo decide el dominio).
      const problemas = await validarEntradaCorreo(entrada);
      if (problemas.length > 0) {
        this.mostrarTodos(problemas);
        return;
      }
      await registrarEntradaCorreo(entrada);
      this.dialogo.close({ nombre: this.nombre.value.trim() });
    } catch (error) {
      this.mostrarTodos([
        esErrorApp(error)
          ? error
          : { tipo: 'tecnico', codigo: 'desconocido', mensaje: 'No se pudo registrar el ingreso.' },
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
    for (const campo of CAMPOS_DEL_NUCLEO) {
      this[campo].setErrors(null);
    }
  }

  /**
   * El mensaje del núcleo junto a su campo; lo que no es de un campo (por
   * ejemplo, que ya está adentro) va junto, arriba de los botones.
   */
  private mostrarTodos(problemas: ErrorApp[]): void {
    const generales: string[] = [];
    for (const problema of problemas) {
      const campo = CAMPOS_DEL_NUCLEO.find((c) => c === problema.campo);
      if (!campo) {
        generales.push(problema.mensaje);
      } else if (!this[campo].errors) {
        this[campo].setErrors({ nucleo: problema.mensaje });
        this[campo].markAsTouched();
      }
    }
    this.errorGeneral.set(generales.length > 0 ? generales.join(' · ') : null);
    enfocarPrimerError(this.anfitrion, this.injector);
  }
}
