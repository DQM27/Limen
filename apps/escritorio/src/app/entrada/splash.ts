import { ChangeDetectionStrategy, Component } from '@angular/core';
import { MatProgressBarModule } from '@angular/material/progress-bar';

/**
 * La pantalla de carga mientras el núcleo responde. Sigue al splash estático
 * de `index.html` (mismo logo y mismo sitio), así que el cambio no se nota.
 */
@Component({
  selector: 'app-splash',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatProgressBarModule],
  template: `
    <div class="splash" data-tauri-drag-region role="status" aria-label="Cargando Limen">
      <img src="logo.svg" alt="" width="112" height="112" />
      <h1>Limen</h1>
      <mat-progress-bar mode="indeterminate" aria-label="Cargando" />
    </div>
  `,
  styles: `
    :host {
      display: block;
      height: 100%;
    }
    .splash {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 12px;
      height: 100%;
      padding: 0 64px;
      box-sizing: border-box;
    }
    img {
      pointer-events: none;
    }
    h1 {
      margin: 0 0 16px;
      font-size: 2.25rem;
      font-weight: 700;
    }
    mat-progress-bar {
      width: 100%;
    }
  `,
})
export class Splash {}
