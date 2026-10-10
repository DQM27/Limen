import { enTauri, invocar } from './tauri';

/**
 * La forma de la ventana: `entrada` es la pequeña y sin marco del splash y
 * del inicio de sesión; `aplicacion`, la maximizada con marco. Sólo le da
 * forma a la ventana; fuera de la app no hace nada.
 */
export type ModoVentana = 'entrada' | 'aplicacion';

export async function modoVentana(modo: ModoVentana): Promise<void> {
  if (enTauri()) {
    await invocar<void>('modo_ventana', { modo });
  }
}

/** Cierra la app. */
export async function salir(): Promise<void> {
  if (enTauri()) {
    await invocar<void>('salir');
  }
}
