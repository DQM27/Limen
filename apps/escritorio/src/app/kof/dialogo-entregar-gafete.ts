import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatAutocompleteModule } from '@angular/material/autocomplete';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { buscarPersonalKof, entregarGafeteKof } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { PersonalKof } from '../nucleo/tipos';

/** Lo que devuelve el modal al entregar el gafete. */
export interface ResultadoEntregaKof {
  nombre: string;
}

/** Cuántas personas se ofrecen al buscar. */
const PERSONAS_OFRECIDAS = 5;

/** Sólo dígitos: lo demás que se escriba se descarta (es formato, no regla). */
function soloDigitos(texto: string): string {
  return texto.replace(/\D/g, '');
}

/** Un campo vacío es 0: el núcleo lo rechaza como gafete inexistente. */
function aNumero(texto: string): number {
  return texto === '' ? 0 : Number.parseInt(texto, 10);
}

/**
 * El modal para entregar el gafete provisional al personal KOF (es su
 * entrada): se busca a la persona por código o nombre y se indica el número
 * del gafete. La lista de coincidencias flota bajo el campo. El núcleo
 * decide si la persona está activa y si el gafete puede prestarse.
 */
@Component({
  selector: 'app-dialogo-entregar-gafete',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatAutocompleteModule,
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
  ],
  templateUrl: './dialogo-entregar-gafete.html',
  styleUrl: './dialogo-entregar-gafete.scss',
})
export class DialogoEntregarGafete {
  private readonly dialogo =
    inject<MatDialogRef<DialogoEntregarGafete, ResultadoEntregaKof>>(MatDialogRef);
  /** Sólo vale la última búsqueda: las que llegan tarde se descartan. */
  private turnoBusqueda = 0;

  /** El texto que se escribe para buscar, o la persona ya elegida de la lista. */
  readonly persona = new FormControl<string | PersonalKof>('', { nonNullable: true });
  readonly gafete = new FormControl('', { nonNullable: true });

  protected readonly opciones = signal<PersonalKof[]>([]);
  protected readonly sinResultados = signal(false);
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
    this.persona.valueChanges.subscribe((valor) => void this.buscar(valor));
  }

  /** Cómo se ve la persona elegida en el campo. */
  protected nombreDePersona = (valor: string | PersonalKof | null): string =>
    typeof valor === 'string' ? valor : valor ? `${valor.nombre} (${valor.codigo_empleado})` : '';

  private async buscar(valor: string | PersonalKof): Promise<void> {
    const turno = ++this.turnoBusqueda;
    if (typeof valor !== 'string') {
      return;
    }
    const texto = valor.trim();
    if (texto === '') {
      this.opciones.set([]);
      this.sinResultados.set(false);
      return;
    }
    const encontradas = await buscarPersonalKof(texto, PERSONAS_OFRECIDAS).catch(() => []);
    if (turno !== this.turnoBusqueda) {
      return;
    }
    this.opciones.set(encontradas);
    this.sinResultados.set(encontradas.length === 0);
  }

  protected alElegir(elegida: PersonalKof): void {
    this.persona.setValue(elegida, { emitEvent: false });
  }

  protected escribirGafete(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const limpio = soloDigitos(campo.value);
    if (limpio !== campo.value) {
      campo.value = limpio;
    }
    this.gafete.setValue(limpio);
  }

  protected async entregar(): Promise<void> {
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      const elegida = this.persona.value;
      await entregarGafeteKof(
        typeof elegida === 'string' ? '' : elegida.id,
        aNumero(this.gafete.value),
      );
      this.dialogo.close({ nombre: typeof elegida === 'string' ? elegida.trim() : elegida.nombre });
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
      this.errorGeneral.set('No se pudo entregar el gafete.');
      return;
    }
    const control =
      error.campo === 'personal_id' ? this.persona : error.campo === 'gafete' ? this.gafete : null;
    if (control) {
      control.setErrors({ nucleo: error.mensaje });
      control.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
