import { Component, inject, OnInit, signal } from '@angular/core';
import { SesionServicio } from '../nucleo/sesion';
import { CambiarClave } from './cambiar-clave';
import { Login } from './login';

/** La carcasa de la app móvil: sin sesión, el login; con sesión, la app. */
@Component({
  selector: 'app-root',
  imports: [CambiarClave, Login],
  template: `
    @if (!listo()) {
      <div class="pantalla"></div>
    } @else {
      @let usuario = sesion.usuario();
      @if (!usuario) {
        <div class="pantalla"><app-login /></div>
      } @else if (usuario.debe_cambiar_clave) {
        <div class="pantalla"><app-cambiar-clave /></div>
      } @else {
        <div class="pantalla">
          <header class="barra-sup">
            <h1>Limen</h1>
            <span class="quien">{{ usuario.nombre }}</span>
            <button class="icono" type="button" aria-label="Salir" (click)="salir()">
              <span class="material-icons" aria-hidden="true">logout</span>
            </button>
          </header>
          <main class="contenido">
            <div class="vacio">Sesión iniciada como {{ usuario.nombre }}.</div>
          </main>
        </div>
      }
    }
  `,
})
export class App implements OnInit {
  protected readonly sesion = inject(SesionServicio);
  protected readonly listo = signal(false);

  async ngOnInit(): Promise<void> {
    try {
      await this.sesion.cargar();
    } finally {
      this.listo.set(true);
    }
  }

  protected salir(): Promise<void> {
    return this.sesion.cerrar();
  }
}
