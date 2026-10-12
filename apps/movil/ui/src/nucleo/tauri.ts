import { invoke, isTauri } from '@tauri-apps/api/core';

/**
 * El error que devuelve cualquier comando del núcleo. La interfaz decide por
 * `codigo` (estable), nunca comparando textos; `mensaje` se muestra tal cual.
 * Los fallos técnicos sólo traen un mensaje genérico: el detalle va al registro.
 */
export interface ErrorApp {
  tipo: 'negocio' | 'tecnico';
  codigo: string;
  mensaje: string;
  /**
   * El campo del formulario que causó el error (la clave del JSON de
   * entrada, por ejemplo `cedula`), para mostrarlo junto a él. `null` si el
   * error no es de un campo. Lo decide el núcleo, no la interfaz.
   */
  campo?: string | null;
}

export function esErrorApp(valor: unknown): valor is ErrorApp {
  if (typeof valor !== 'object' || valor === null) {
    return false;
  }
  const candidato = valor as Record<string, unknown>;
  return (
    (candidato['tipo'] === 'negocio' || candidato['tipo'] === 'tecnico') &&
    typeof candidato['codigo'] === 'string' &&
    typeof candidato['mensaje'] === 'string'
  );
}

/** Si la interfaz corre dentro de la app (y no en un navegador suelto). */
export function enTauri(): boolean {
  return isTauri();
}

/**
 * Llama a un comando del núcleo. Tauri convierte los nombres de los
 * parámetros a camelCase (`ingreso_id` se envía como `ingresoId`); los campos
 * de los objetos que viajan dentro se quedan en snake_case, igual que el JSON.
 *
 * Si el comando falla, rechaza con un {@link ErrorApp}.
 */
export function invocar<T>(comando: string, argumentos?: Record<string, unknown>): Promise<T> {
  return invoke<T>(comando, argumentos);
}
