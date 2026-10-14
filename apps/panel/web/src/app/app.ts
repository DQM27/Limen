import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { MatIconModule } from '@angular/material/icon';
import { MatListModule } from '@angular/material/list';
import { MatTooltipModule } from '@angular/material/tooltip';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { SECCIONES } from './navegacion';

/**
 * El cascarón del panel de administración: el menú lateral con todas las
 * secciones y, al lado, la pantalla de la sección elegida. No decide nada: cada
 * pantalla muestra lo que responde el Worker, y las reglas son del núcleo.
 */
@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatIconModule,
    MatListModule,
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
  protected readonly expandida = signal(true);

  protected alternar(): void {
    this.expandida.update((valor) => !valor);
  }
}
