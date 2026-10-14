/** Una entrada del menú lateral. Sin `ruta`, la pantalla todavía no existe. */
export interface Seccion {
  etiqueta: string;
  /** Ícono de Material (por su nombre). */
  icono: string;
  ruta?: string;
}

/**
 * Todas las secciones previstas del panel, para que se vea su forma. Cada
 * pantalla nueva sólo agrega su `ruta` (y su entrada en `app.routes.ts`).
 */
export const SECCIONES: readonly Seccion[] = [
  { etiqueta: 'Equipos', icono: 'devices', ruta: '/equipos' },
  { etiqueta: 'Usuarios', icono: 'group', ruta: '/usuarios' },
  { etiqueta: 'Historiales', icono: 'history' },
  { etiqueta: 'Contratistas', icono: 'engineering', ruta: '/contratistas' },
];
