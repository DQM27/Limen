import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { MatIconModule } from '@angular/material/icon';
import { MatListModule } from '@angular/material/list';
import { MatTooltipModule } from '@angular/material/tooltip';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { AvisosPanel } from './compartido/avisos';
import { AlertasEnVivo } from './en-vivo/alertas-en-vivo';
import { EnVivo, type EstadoEnVivo } from './en-vivo/en-vivo';
import { SECCIONES } from './navegacion';

/** Cómo se le dice a quien mira el estado del tiempo real, y qué significa. */
const TEXTO_EN_VIVO: Record<EstadoEnVivo, { etiqueta: string; icono: string; ayuda: string }> = {
  en_vivo: {
    etiqueta: 'En vivo',
    icono: 'sensors',
    ayuda: 'Los cambios llegan al instante.',
  },
  conectando: {
    etiqueta: 'Conectando…',
    icono: 'sync',
    ayuda: 'Conectando con la base para recibir los cambios al instante.',
  },
  sin_conexion: {
    etiqueta: 'Reconectando…',
    icono: 'sync_problem',
    ayuda:
      'Se perdió la conexión en vivo; se reintenta solo. Mientras tanto la página se actualiza cada 15 segundos.',
  },
  no_disponible: {
    etiqueta: 'Sin tiempo real',
    icono: 'sensors_off',
    ayuda:
      'El tiempo real no está configurado en el servidor; la página se actualiza cada 15 segundos.',
  },
};

/**
 * El cascarón del panel de administración: el menú lateral con todas las
 * secciones y, al lado, la pantalla de la sección elegida. No decide nada: cada
 * pantalla muestra lo que responde el Worker, y las reglas son del núcleo.
 */
@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    AvisosPanel,
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
  protected readonly enVivo = inject(EnVivo);
  protected readonly textoEnVivo = TEXTO_EN_VIVO;

  constructor() {
    // Los avisos en vivo (usuario desactivado, equipo sin señal, nuevo ingreso…) valen en toda la
    // página, esté abierta la sección que esté.
    inject(AlertasEnVivo).iniciar();
  }

  protected alternar(): void {
    this.expandida.update((valor) => !valor);
  }
}
