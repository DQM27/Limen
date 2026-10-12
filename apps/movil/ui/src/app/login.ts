import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { SesionServicio } from '../nucleo/sesion';
import { mensajeDe } from './mensaje-error';

/** Inicio de sesión: cédula y clave. Quién puede entrar lo decide el núcleo. */
@Component({
  selector: 'app-login',
  imports: [FormsModule],
  template: `
    <form class="entrada" (ngSubmit)="entrar()">
      <div class="logo" aria-hidden="true">L</div>
      <h1>Limen</h1>
      <p class="sub">Control de acceso</p>
      <input
        class="campo"
        name="cedula"
        inputmode="numeric"
        autocomplete="username"
        placeholder="Cédula"
        [(ngModel)]="cedula"
      />
      <input
        class="campo"
        name="clave"
        type="password"
        autocomplete="current-password"
        placeholder="Clave"
        [(ngModel)]="clave"
      />
      @if (error()) {
        <div class="error" role="alert">{{ error() }}</div>
      }
      <button class="boton" type="submit" [disabled]="!cedula || !clave || ocupado()">
        {{ ocupado() ? 'Entrando…' : 'Ingresar' }}
      </button>
    </form>
  `,
})
export class Login {
  private readonly sesion = inject(SesionServicio);

  protected cedula = '';
  protected clave = '';
  protected readonly error = signal('');
  protected readonly ocupado = signal(false);

  protected async entrar(): Promise<void> {
    this.error.set('');
    this.ocupado.set(true);
    try {
      await this.sesion.iniciar(this.cedula.trim(), this.clave);
    } catch (e) {
      this.error.set(mensajeDe(e));
    } finally {
      this.ocupado.set(false);
    }
  }
}
