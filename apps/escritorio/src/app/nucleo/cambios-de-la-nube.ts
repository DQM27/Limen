import { DestroyRef, Injectable, inject } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { enTauri } from './tauri';

/** El evento que manda el núcleo cuando bajan cambios de la nube. */
export const EVENTO_CAMBIOS_DE_LA_NUBE = 'nube-cambios';

/**
 * Avisa a las pantallas cuando bajaron cambios de la nube (lo que registran
 * los demás equipos), para que recarguen lo que muestran. Qué cambió y cómo
 * se aplica lo decide el núcleo: la pantalla sólo vuelve a pedir sus datos.
 */
@Injectable({ providedIn: 'root' })
export class CambiosDeLaNube {
  private readonly escuchas = new Set<() => void>();

  constructor() {
    if (enTauri()) {
      // Si no se puede escuchar (fuera de la app), la pantalla trabaja igual.
      listen(EVENTO_CAMBIOS_DE_LA_NUBE, () => this.avisar()).catch(() => undefined);
    }
  }

  /**
   * Llama a `recargar` cada vez que bajen cambios de la nube, mientras viva
   * el componente que lo pide. Se llama desde el constructor del componente.
   */
  alLlegar(recargar: () => void, destruccion: DestroyRef = inject(DestroyRef)): void {
    this.escuchas.add(recargar);
    destruccion.onDestroy(() => this.escuchas.delete(recargar));
  }

  /** Avisa a todas las pantallas abiertas. */
  avisar(): void {
    for (const recargar of this.escuchas) {
      recargar();
    }
  }
}
