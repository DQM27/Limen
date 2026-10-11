import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatListModule } from '@angular/material/list';
import { MatTooltipModule } from '@angular/material/tooltip';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { DURACION_MINIMA_SPLASH_MS } from './entrada/duracion-splash';
import { PantallaEntrada } from './entrada/pantalla-entrada';
import { Splash } from './entrada/splash';
import { registrarIconos } from './iconos';
import { SECCIONES } from './navegacion';
import { SesionServicio } from './nucleo/sesion';
import { enTauri } from './nucleo/tauri';
import { modoVentana } from './nucleo/ventana';

@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatButtonModule,
    MatIconModule,
    MatListModule,
    MatTooltipModule,
    PantallaEntrada,
    RouterLink,
    RouterLinkActive,
    RouterOutlet,
    Splash,
  ],
  templateUrl: './app.html',
  styleUrl: './app.scss',
})
export class App {
  protected readonly secciones = SECCIONES;
  private readonly sesion = inject(SesionServicio);
  protected readonly operador = this.sesion.usuario;
  private readonly splashVisto = signal(false);
  protected readonly enTauri = enTauri();

  /**
   * Qué se ve: el splash mientras el núcleo responde, la entrada (inicio de
   * sesión o cambio de clave) sin sesión o con clave temporal, y la app. Fuera
   * de la app (en el navegador) no hay sesión que pedir.
   */
  protected readonly fase = computed<'cargando' | 'entrada' | 'aplicacion'>(() => {
    if (!this.enTauri) {
      return 'aplicacion';
    }
    if (!this.sesion.cargada() || !this.splashVisto()) {
      return 'cargando';
    }
    const usuario = this.sesion.usuario();
    return usuario && !usuario.debe_cambiar_clave ? 'aplicacion' : 'entrada';
  });

  /** La barra lateral completa (con texto) o reducida a íconos. */
  protected readonly expandida = signal(true);

  constructor() {
    registrarIconos();
    void this.sesion.cargar();
    // El splash se ve un mínimo, aunque el núcleo responda al instante: sin
    // esto pasa tan fugaz que no se alcanza a ver la marca.
    const minimo = inject(DURACION_MINIMA_SPLASH_MS);
    if (minimo > 0) {
      setTimeout(() => this.splashVisto.set(true), minimo);
    } else {
      this.splashVisto.set(true);
    }
    // La ventana toma la forma de cada fase: pequeña y sin marco para el splash
    // y la entrada; maximizada y con marco para la app.
    effect(() => {
      void modoVentana(this.fase() === 'aplicacion' ? 'aplicacion' : 'entrada').catch(() => {});
    });
  }

  protected cerrarSesion(): void {
    void this.sesion.cerrar();
  }

  protected alternar(): void {
    this.expandida.update((valor) => !valor);
  }

  /** Lleva el foco al contenido principal (enlace "Saltar al contenido"). */
  protected irAlContenido(evento: Event): void {
    evento.preventDefault();
    document.getElementById('contenido')?.focus();
  }
}
