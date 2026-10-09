import type { FilaContratista } from './tipos';
import { invocar } from './tauri';

/**
 * Los comandos del núcleo con su forma tipada: un método por comando de
 * Tauri, sin lógica. Es el único lugar que conoce los nombres de los
 * comandos y de sus parámetros.
 */

export function listarContratistas(): Promise<FilaContratista[]> {
  return invocar<FilaContratista[]>('listar_contratistas');
}
