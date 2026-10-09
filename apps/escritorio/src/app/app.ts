import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatListModule } from '@angular/material/list';
import { MatSidenavModule } from '@angular/material/sidenav';
import { MatTooltipModule } from '@angular/material/tooltip';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { SECCIONES } from './navegacion';
import { OperadorServicio } from './nucleo/operador';
import { enTauri } from './nucleo/tauri';

@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatButtonModule,
    MatIconModule,
    MatListModule,
    MatSidenavModule,
    MatTooltipModule,
    RouterLink,
    RouterLinkActive,
    RouterOutlet,
  ],
  templateUrl: './app.html',
  styleUrl: './app.scss',
})
export class App {
  protected readonly secciones = SECCIONES;
  protected readonly operador = inject(OperadorServicio).operador;
  protected readonly enTauri = enTauri();

  /** La barra lateral completa (con texto) o reducida a íconos. */
  protected readonly expandida = signal(true);

  constructor() {
    void inject(OperadorServicio).cargar();
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
