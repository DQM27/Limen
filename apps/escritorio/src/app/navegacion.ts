/** Una entrada de la barra lateral. Sin `ruta`, la pantalla todavía no existe. */
export interface Seccion {
  etiqueta: string;
  /** Ícono de Material (por su nombre). */
  icono: string;
  /** Un ícono propio (ver iconos.ts); si está, se usa en vez de `icono`. */
  iconoPropio?: string;
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
  { etiqueta: 'KOF', icono: 'local_drink', iconoPropio: 'kof' },
  { etiqueta: 'Gafetes', icono: 'badge' },
  { etiqueta: 'Historial', icono: 'history' },
];
