import { ChangeDetectionStrategy, Component, effect, input, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { placaAlEscribir } from '../nucleo/comandos';
import type { Medio } from '../nucleo/tipos';
import { NormalizadorAlEscribir } from './escribir-normalizado';

/** Sólo dígitos: lo demás que se escriba se descarta (es formato, no regla). */
function soloDigitos(texto: string): string {
  return texto.replace(/\D/g, '');
}

/**
 * Lo que comparten los ingresos de proveedor y de correo: cómo llegó la
 * persona (a pie o en vehículo), la placa si va en vehículo y el número de
 * gafete. Los controles son del formulario que usa esta pieza, que también
 * pone ahí los errores del núcleo (`setErrors({ nucleo: mensaje })`).
 */
@Component({
  selector: 'app-campos-de-ingreso',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonToggleModule, MatFormFieldModule, MatInputModule, ReactiveFormsModule],
  templateUrl: './campos-de-ingreso.html',
  styleUrl: './campos-de-ingreso.scss',
})
export class CamposDeIngreso {
  readonly medio = input.required<FormControl<Medio>>();
  readonly placa = input.required<FormControl<string>>();
  readonly gafete = input.required<FormControl<string>>();
  readonly etiquetaGafete = input('Número de gafete');

  protected readonly enVehiculo = signal(false);
  private readonly normalizaPlaca = new NormalizadorAlEscribir();

  constructor() {
    // La placa se pide sólo en vehículo; se sigue al control sin importar quién lo cambie.
    effect((limpiar) => {
      const control = this.medio();
      this.enVehiculo.set(control.value === 'VEHICULO');
      const suscripcion = control.valueChanges.subscribe((medio) =>
        this.enVehiculo.set(medio === 'VEHICULO'),
      );
      limpiar(() => suscripcion.unsubscribe());
    });
  }

  /** La placa se ve en mayúsculas mientras se escribe (la forma la da el dominio). */
  protected escribirPlaca(evento: Event): void {
    void this.normalizaPlaca.alEscribir(evento, this.placa(), placaAlEscribir);
  }

  protected escribirGafete(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const limpio = soloDigitos(campo.value);
    if (limpio !== campo.value) {
      campo.value = limpio;
    }
    this.gafete().setValue(limpio);
  }
}
