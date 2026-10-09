/** Una entrada de la barra lateral. */
export interface Seccion {
  etiqueta: string;
  icono: string;
  ruta: string;
}

/**
 * Sólo lo que ya funciona: una sección sin pantalla se ve como un botón roto.
 * Cada pantalla nueva agrega aquí su entrada (previstas: Dentro, Proveedores,
 * Correo, Personal KOF, Gafetes e Historial).
 */
export const SECCIONES: readonly Seccion[] = [
  { etiqueta: 'Contratistas', icono: 'engineering', ruta: '/contratistas' },
];
