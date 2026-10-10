/** Una entrada de la barra lateral. Sin `ruta`, la pantalla todavía no existe. */
export interface Seccion {
  etiqueta: string;
  icono: string;
  ruta?: string;
}

/**
 * Todas las secciones previstas, para que se vea la forma de la app. Cada
 * pantalla nueva sólo agrega su `ruta`.
 */
export const SECCIONES: readonly Seccion[] = [
  { etiqueta: 'Dentro', icono: 'groups' },
  { etiqueta: 'Contratistas', icono: 'engineering', ruta: '/contratistas' },
  { etiqueta: 'Proveedores', icono: 'local_shipping' },
  { etiqueta: 'Correo', icono: 'mail' },
  { etiqueta: 'Personal KOF', icono: 'badge' },
  { etiqueta: 'Gafetes', icono: 'confirmation_number' },
  { etiqueta: 'Historial', icono: 'history' },
];
