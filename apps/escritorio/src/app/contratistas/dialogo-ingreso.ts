import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatAutocompleteModule } from '@angular/material/autocomplete';
import { MatButtonModule } from '@angular/material/button';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import { fechaCorta } from '../compartido/formato';
import { etiquetaDeVia } from '../dentro/etiquetas';
import {
  buscarParaIngreso,
  placaAlEscribir,
  prepararIngreso,
  registrarEntradaContratista,
} from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { Acceso, CandidatoIngreso, EntradaContratista, Medio } from '../nucleo/tipos';
import { etiquetaDeAcceso, etiquetaDeTipo } from './etiquetas';

/** Con `contratistaId` el modal abre directo en esa persona; sin él, empieza por buscarla. */
export interface DatosIngreso {
  contratistaId?: string;
}

/** Lo que devuelve el modal al registrar el ingreso. */
export interface ResultadoIngreso {
  nombre: string;
  acceso: Acceso;
}

/** Cuántas personas se ofrecen al buscar. */
const RESULTADOS_OFRECIDOS = 5;

/** Sólo dígitos: lo demás que se escriba se descarta (es formato, no regla). */
function soloDigitos(texto: string): string {
  return texto.replace(/\D/g, '');
}

/**
 * El modal de ingreso de un contratista: se busca a la persona (por cédula o
 * nombre) y se ve su ficha con la decisión que ya tomó el núcleo (puede
 * entrar o no, y por qué); si puede, se indica el medio, la placa si va en
 * vehículo y el gafete (un número o «Sin gafete»). La interfaz no decide
 * nada: el núcleo exige cada dato y su respuesta se muestra junto a su campo.
 */
@Component({
  selector: 'app-dialogo-ingreso',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatAutocompleteModule,
    MatButtonModule,
    MatButtonToggleModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
  ],
  templateUrl: './dialogo-ingreso.html',
  styleUrl: './dialogo-ingreso.scss',
})
export class DialogoIngreso {
  private readonly dialogo = inject<MatDialogRef<DialogoIngreso, ResultadoIngreso>>(MatDialogRef);
  private readonly datos = inject<DatosIngreso | null>(MAT_DIALOG_DATA, { optional: true });
  private readonly normalizaPlaca = new NormalizadorAlEscribir();
  /** Sólo vale la última búsqueda: las que llegan tarde se descartan. */
  private turnoBusqueda = 0;

  protected readonly etiquetaDeAcceso = etiquetaDeAcceso;
  protected readonly etiquetaDeTipo = etiquetaDeTipo;
  protected readonly etiquetaDeVia = etiquetaDeVia;
  protected readonly fechaCorta = fechaCorta;

  /** La persona elegida; sin ella se muestra el buscador. */
  protected readonly candidato = signal<CandidatoIngreso | null>(null);
  readonly busqueda = new FormControl('', { nonNullable: true });
  protected readonly resultados = signal<CandidatoIngreso[]>([]);
  protected readonly sinResultados = signal(false);

  readonly medio = new FormControl<Medio>('A_PIE', { nonNullable: true });
  readonly placa = new FormControl('', { nonNullable: true });
  readonly gafete = new FormControl('', { nonNullable: true });
  readonly sinGafete = new FormControl(false, { nonNullable: true });

  protected readonly enviando = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);

  /** Se abrió directo en una persona: no hay buscador al que volver. */
  protected readonly abiertoEnPersona = this.datos?.contratistaId !== undefined;
  protected readonly enVehiculo = signal(false);

  /** Puede registrarse: el núcleo dijo que puede entrar. */
  protected readonly puedeEntrar = computed(() => this.candidato()?.puede_entrar === true);

  constructor() {
    // Un clic fuera no lo cierra (no se pierde lo escrito por accidente);
    // Escape sí, como cualquier diálogo.
    this.dialogo.keydownEvents().subscribe((evento) => {
      if (evento.key === 'Escape') {
        this.cancelar();
      }
    });
    this.busqueda.valueChanges.subscribe((texto) => void this.buscar(texto));
    this.medio.valueChanges.subscribe((medio) => this.enVehiculo.set(medio === 'VEHICULO'));
    const id = this.datos?.contratistaId;
    if (id !== undefined) {
      void this.abrirFicha(id);
    }
  }

  /** Cómo se ve cada resultado de la búsqueda: lo que dijo el núcleo, traducido. */
  protected estado(candidato: CandidatoIngreso): string {
    if (candidato.adentro_por !== null) {
      return 'Ya está adentro';
    }
    if (!candidato.puede_entrar) {
      return candidato.motivo?.mensaje ?? 'No puede entrar';
    }
    return candidato.acceso ? etiquetaDeAcceso(candidato.acceso) : 'Puede entrar';
  }

  private async buscar(texto: string): Promise<void> {
    const turno = ++this.turnoBusqueda;
    const buscado = texto.trim();
    if (buscado === '') {
      this.resultados.set([]);
      this.sinResultados.set(false);
      return;
    }
    const encontrados = await buscarParaIngreso(buscado, RESULTADOS_OFRECIDOS).catch(() => []);
    if (turno !== this.turnoBusqueda) {
      return;
    }
    this.resultados.set(encontrados);
    this.sinResultados.set(encontrados.length === 0);
  }

  /** Un aviso de PRAIND por vencer, si el núcleo dice que lo hay. */
  protected advertencia(candidato: CandidatoIngreso): string | null {
    return candidato.acceso?.resultado === 'PERMITIDO_CON_ADVERTENCIA'
      ? etiquetaDeAcceso(candidato.acceso)
      : null;
  }

  /** Al elegir de la lista, el campo no muestra nada: la persona pasa a la ficha. */
  protected sinTexto = (): string => '';

  /**
   * Enter en el buscador elige el primer resultado (el que mejor coincide):
   * sirve al leer una cédula, aunque la lista todavía no se haya desplegado.
   * Si la lista ya estaba abierta, ella misma elige la opción activa.
   */
  protected async alPulsarEnter(evento: Event): Promise<void> {
    evento.preventDefault();
    await this.buscar(this.busqueda.value);
    const primero = this.resultados()[0];
    if (primero && this.candidato() === null) {
      this.elegir(primero);
    }
  }

  protected elegir(candidato: CandidatoIngreso): void {
    this.errorGeneral.set(null);
    this.candidato.set(candidato);
  }

  /** Vuelve al buscador para elegir a otra persona. */
  protected cambiar(): void {
    this.candidato.set(null);
    this.errorGeneral.set(null);
  }

  private async abrirFicha(id: string): Promise<void> {
    try {
      this.candidato.set(await prepararIngreso(id));
    } catch (error) {
      this.errorGeneral.set(esErrorApp(error) ? error.mensaje : 'No se pudo abrir la ficha.');
    }
  }

  /** La placa se ve en mayúsculas mientras se escribe (la forma la da el dominio). */
  protected escribirPlaca(evento: Event): void {
    void this.normalizaPlaca.alEscribir(evento, this.placa, placaAlEscribir);
  }

  /** Escribir un número desmarca «Sin gafete»: son una cosa o la otra. */
  protected escribirGafete(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const limpio = soloDigitos(campo.value);
    if (limpio !== campo.value) {
      campo.value = limpio;
    }
    this.gafete.setValue(limpio);
    if (limpio !== '') {
      this.sinGafete.setValue(false);
    }
  }

  /** Marcar «Sin gafete» borra el número: son una cosa o la otra. */
  protected alMarcarSinGafete(marcado: boolean): void {
    this.sinGafete.setValue(marcado);
    if (marcado) {
      this.gafete.setValue('');
      this.gafete.disable();
    } else {
      this.gafete.enable();
    }
  }

  protected async registrar(): Promise<void> {
    const persona = this.candidato();
    if (!persona || this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      const entrada: EntradaContratista = {
        contratista_id: persona.id,
        medio: this.medio.value,
        placa: this.medio.value === 'VEHICULO' ? this.placa.value : null,
        gafete: this.gafete.value === '' ? null : Number.parseInt(this.gafete.value, 10),
        sin_gafete: this.sinGafete.value,
      };
      const registrada = await registrarEntradaContratista(entrada);
      this.dialogo.close({ nombre: persona.nombre, acceso: registrada.acceso });
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }

  /** Un error de la placa o del gafete va junto a su campo; los demás, arriba de los botones. */
  private mostrar(error: unknown): void {
    if (!esErrorApp(error)) {
      this.errorGeneral.set('No se pudo registrar el ingreso.');
      return;
    }
    const control =
      error.campo === 'placa' ? this.placa : error.campo === 'gafete' ? this.gafete : null;
    if (control) {
      control.setErrors({ nucleo: error.mensaje });
      control.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
