import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { Avisos } from '../compartido/avisos';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import { buscarPersonalKof, editarPersonalKof, nombrePersonaAlEscribir } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { PersonalKof } from '../nucleo/tipos';

/** Cuántas personas se muestran al buscar: el área de la lista tiene alto fijo. */
const PERSONAS_MOSTRADAS = 6;

/**
 * El modal para administrar al personal KOF: se busca por código o nombre y,
 * de cada persona, se corrige el nombre o se la desactiva (nunca se borra: K2).
 * El código de empleado no se cambia, es su identidad. El núcleo decide qué
 * vale; aquí sólo se muestra su respuesta.
 */
@Component({
  selector: 'app-dialogo-administrar-personal',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    ReactiveFormsModule,
  ],
  templateUrl: './dialogo-administrar-personal.html',
  styleUrl: './dialogo-administrar-personal.scss',
})
export class DialogoAdministrarPersonal {
  private readonly dialogo = inject<MatDialogRef<DialogoAdministrarPersonal>>(MatDialogRef);
  private readonly avisos = inject(Avisos);
  private readonly normalizaNombre = new NormalizadorAlEscribir();
  /** Sólo vale la última búsqueda: las que llegan tarde se descartan. */
  private turnoBusqueda = 0;

  readonly busqueda = new FormControl('', { nonNullable: true });
  /** El nombre nuevo de la persona que se está corrigiendo. */
  readonly nombre = new FormControl('', { nonNullable: true });

  protected readonly personas = signal<PersonalKof[]>([]);
  protected readonly buscado = signal(false);
  /** El id de la persona cuyo nombre se está corrigiendo, si hay una. */
  protected readonly editando = signal<string | null>(null);
  protected readonly enviando = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);

  constructor() {
    // Escape cierra el modal; si se estaba corrigiendo un nombre, sólo cancela eso.
    this.dialogo.keydownEvents().subscribe((evento) => {
      if (evento.key !== 'Escape') {
        return;
      }
      if (this.editando() !== null) {
        this.cancelarEdicion();
      } else {
        this.cerrar();
      }
    });
    this.busqueda.valueChanges.subscribe((texto) => void this.buscar(texto));
  }

  private async buscar(valor: string): Promise<void> {
    const turno = ++this.turnoBusqueda;
    this.cancelarEdicion();
    const texto = valor.trim();
    if (texto === '') {
      this.personas.set([]);
      this.buscado.set(false);
      return;
    }
    const encontradas = await buscarPersonalKof(texto, PERSONAS_MOSTRADAS).catch(() => []);
    if (turno !== this.turnoBusqueda) {
      return;
    }
    this.personas.set(encontradas);
    this.buscado.set(true);
  }

  protected escribirNombre(evento: Event): void {
    void this.normalizaNombre.alEscribir(evento, this.nombre, nombrePersonaAlEscribir);
  }

  protected corregir(persona: PersonalKof): void {
    this.errorGeneral.set(null);
    this.nombre.setValue(persona.nombre);
    this.nombre.setErrors(null);
    this.editando.set(persona.id);
  }

  protected cancelarEdicion(): void {
    this.editando.set(null);
    this.nombre.setErrors(null);
  }

  protected async guardarNombre(persona: PersonalKof): Promise<void> {
    await this.guardar(persona, this.nombre.value, persona.activo, 'Nombre corregido');
  }

  protected async alternarActivo(persona: PersonalKof): Promise<void> {
    await this.guardar(
      persona,
      persona.nombre,
      !persona.activo,
      persona.activo ? 'Persona desactivada' : 'Persona activada',
    );
  }

  private async guardar(
    persona: PersonalKof,
    nombre: string,
    activo: boolean,
    titulo: string,
  ): Promise<void> {
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      await editarPersonalKof(persona.id, nombre, activo);
      const nuevoNombre = nombre.trim();
      this.personas.update((lista) =>
        lista.map((p) => (p.id === persona.id ? { ...p, nombre: nuevoNombre, activo } : p)),
      );
      this.cancelarEdicion();
      this.avisos.exito(nuevoNombre, titulo);
    } catch (error) {
      this.mostrar(error, persona);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cerrar(): void {
    this.dialogo.close();
  }

  /** El mensaje del núcleo junto al nombre que se corrige; si no, arriba de la lista. */
  private mostrar(error: unknown, persona: PersonalKof): void {
    const mensaje = esErrorApp(error) ? error.mensaje : 'No se pudo guardar el cambio.';
    if (esErrorApp(error) && error.campo === 'nombre' && this.editando() === persona.id) {
      this.nombre.setErrors({ nucleo: mensaje });
      this.nombre.markAsTouched();
    } else {
      this.errorGeneral.set(mensaje);
    }
  }
}
