import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatAutocompleteModule } from '@angular/material/autocomplete';
import { MatButtonModule } from '@angular/material/button';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { buscarParaIngreso, buscarPersonalKof, cambiarGafete } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { Cambio, CambioGafete, Gafete } from '../nucleo/tipos';
import { etiquetaDeEstadoGafete, etiquetaDeTipoGafete, textoAsignado } from './etiquetas';

export interface DatosGestionGafete {
  gafete: Gafete;
}

/** Lo que devuelve el modal al cambiar el gafete: qué se hizo y lo que cambió. */
export interface ResultadoGestionGafete {
  cambio: CambioGafete['cambio'];
  cambios: Cambio[];
}

/** Alguien del catálogo que se ofrece como último portador. */
interface Persona {
  id: string;
  etiqueta: string;
}

/** Cuántas personas se ofrecen al escribir. */
const PERSONAS_OFRECIDAS = 8;

/**
 * El modal de un gafete: según su estado ofrece lo que corresponde (marcarlo
 * perdido o darlo de baja si está disponible; pagado o apareció si está
 * perdido). Qué se puede de verdad lo exige el núcleo: aquí sólo se pide lo
 * que hace falta y se muestra su respuesta.
 */
@Component({
  selector: 'app-dialogo-gestion-gafete',
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
  templateUrl: './dialogo-gestion-gafete.html',
  styleUrl: './dialogo-gestion-gafete.scss',
})
export class DialogoGestionGafete {
  private readonly dialogo =
    inject<MatDialogRef<DialogoGestionGafete, ResultadoGestionGafete>>(MatDialogRef);
  protected readonly gafete = inject<DatosGestionGafete>(MAT_DIALOG_DATA).gafete;

  protected readonly etiquetaTipo = etiquetaDeTipoGafete(this.gafete.tipo);
  protected readonly etiquetaEstado = etiquetaDeEstadoGafete(this.gafete);
  protected readonly asignado = textoAsignado(this.gafete);

  /** El último portador: se elige de un catálogo (contratista, KOF) o se escribe la cédula. */
  protected readonly eligeDeCatalogo =
    this.gafete.tipo === 'CONTRATISTA' || this.gafete.tipo === 'PROVISIONAL_KOF';
  readonly portador = new FormControl<string | Persona>('', { nonNullable: true });
  protected readonly opciones = signal<Persona[]>([]);

  protected readonly perdiendo = signal(false);
  protected readonly confirmandoBaja = signal(false);
  protected readonly enviando = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);

  /** Lo que se ofrece según el estado (el núcleo lo vuelve a exigir). */
  protected readonly situacion = computed<'prestado' | 'disponible' | 'perdido' | 'de_baja'>(() => {
    if (this.gafete.prestado) {
      return 'prestado';
    }
    switch (this.gafete.estado) {
      case 'DISPONIBLE':
        return 'disponible';
      case 'PERDIDO':
        return 'perdido';
      case 'DE_BAJA':
        return 'de_baja';
    }
  });

  constructor() {
    this.dialogo.keydownEvents().subscribe((evento) => {
      if (evento.key === 'Escape') {
        this.cerrar();
      }
    });
    this.portador.valueChanges.subscribe((valor) => void this.ofrecerPersonas(valor));
  }

  protected nombreDePersona = (valor: string | Persona | null): string =>
    typeof valor === 'string' ? valor : (valor?.etiqueta ?? '');

  /** Al escribir, se ofrecen las personas que coinciden (el núcleo decide cuáles). */
  private async ofrecerPersonas(valor: string | Persona): Promise<void> {
    if (!this.eligeDeCatalogo || typeof valor !== 'string') {
      return;
    }
    const texto = valor.trim();
    if (texto === '') {
      this.opciones.set([]);
      return;
    }
    if (this.gafete.tipo === 'CONTRATISTA') {
      const encontrados = await buscarParaIngreso(texto, PERSONAS_OFRECIDAS).catch(() => []);
      this.opciones.set(
        encontrados.map((c) => ({ id: c.id, etiqueta: `${c.nombre} (${c.cedula})` })),
      );
    } else {
      const encontrados = await buscarPersonalKof(texto, PERSONAS_OFRECIDAS).catch(() => []);
      this.opciones.set(
        encontrados.map((p) => ({ id: p.id, etiqueta: `${p.nombre} (${p.codigo_empleado})` })),
      );
    }
  }

  protected empezarPerdido(): void {
    this.confirmandoBaja.set(false);
    this.perdiendo.set(true);
  }

  protected empezarBaja(): void {
    this.perdiendo.set(false);
    this.confirmandoBaja.set(true);
  }

  protected async marcarPerdido(): Promise<void> {
    const valor = this.portador.value;
    const elegido = typeof valor === 'string' ? null : valor;
    const cambio: CambioGafete = {
      cambio: 'PERDIDO',
      portador_contratista_id: this.gafete.tipo === 'CONTRATISTA' ? (elegido?.id ?? null) : null,
      portador_personal_id: this.gafete.tipo === 'PROVISIONAL_KOF' ? (elegido?.id ?? null) : null,
      portador_cedula: this.eligeDeCatalogo ? null : typeof valor === 'string' ? valor : '',
    };
    await this.enviar(cambio);
  }

  protected async resolver(tipo: 'PAGADO' | 'APARECIDO' | 'DE_BAJA'): Promise<void> {
    await this.enviar({
      cambio: tipo,
      portador_contratista_id: null,
      portador_cedula: null,
      portador_personal_id: null,
    });
  }

  private async enviar(cambio: CambioGafete): Promise<void> {
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      const cambios = await cambiarGafete(this.gafete.tipo, this.gafete.numero, cambio);
      this.dialogo.close({ cambio: cambio.cambio, cambios });
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cerrar(): void {
    this.dialogo.close();
  }

  /** Un error del último portador va junto a su campo; los demás, arriba de los botones. */
  private mostrar(error: unknown): void {
    if (!esErrorApp(error)) {
      this.errorGeneral.set('No se pudo cambiar el gafete.');
      return;
    }
    if (error.campo === 'portador') {
      this.portador.setErrors({ nucleo: error.mensaje });
      this.portador.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
