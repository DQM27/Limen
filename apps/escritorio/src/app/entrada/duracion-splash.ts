import { InjectionToken } from '@angular/core';

/**
 * Lo mínimo que se ve el splash, en milisegundos, aunque el núcleo responda
 * antes: sin un piso, en un equipo rápido pasa tan fugaz que no se alcanza a
 * ver la marca. Las pruebas lo ponen en 0.
 */
export const DURACION_MINIMA_SPLASH_MS = new InjectionToken<number>('DURACION_MINIMA_SPLASH_MS', {
  providedIn: 'root',
  factory: () => 3000,
});
