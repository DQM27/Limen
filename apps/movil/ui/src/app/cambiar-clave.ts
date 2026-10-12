import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { SesionServicio } from '../nucleo/sesion';
import { mensajeDe } from './mensaje-error';

/** Con una clave temporal, lo único permitido es cambiarla. */
@Component({
  selector: 'app-cambiar-clave',
  imports: [FormsModule],
  template: `
    <form class="entrada" (ngSubmit)="guardar()">
      <h1>Cambie su clave</h1>
      <p class="sub">La clave actual es temporal.</p>
      <input
        class="campo"
        name="actual"
        type="password"
        autocomplete="current-password"
        placeholder="Clave actual"
        [(ngModel)]="actual"
      />
      <input
        class="campo"
        name="nueva"
        type="password"
        autocomplete="new-password"
        placeholder="Clave nueva"
        [(ngModel)]="nueva"
      />
      @if (error()) {
        <div class="error" role="alert">{{ error() }}</div>
      }
      <button class="boton" type="submit" [disabled]="!actual || !nueva || ocupado()">
        Guardar
      </button>
      <button class="boton secundario" type="button" (click)="salir()">Salir</button>
    </form>
  `,
})
export class CambiarClave {
  private readonly sesion = inject(SesionServicio);

  protected actual = '';
  protected nueva = '';
  protected readonly error = signal('');
  protected readonly ocupado = signal(false);

  protected async guardar(): Promise<void> {
    this.error.set('');
    this.ocupado.set(true);
    try {
      await this.sesion.cambiarClave(this.actual, this.nueva);
    } catch (e) {
      this.error.set(mensajeDe(e));
    } finally {
      this.ocupado.set(false);
    }
  }

  protected salir(): Promise<void> {
    return this.sesion.cerrar();
  }
}
