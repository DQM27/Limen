import { esErrorApp } from '../nucleo/tauri';

/** El texto que se le muestra a quien opera cuando un comando falla. */
export function mensajeDe(error: unknown): string {
  if (esErrorApp(error)) {
    return error.mensaje;
  }
  return 'No se pudo completar la operación.';
}
