import type { TipoIngreso } from '../panel';

/** El texto con que se muestra el código del tipo de ingreso (el mismo del escritorio). */
export function etiquetaDeTipo(tipo: TipoIngreso): string {
  return tipo === 'IN_HOUSE' ? 'IN HOUSE' : 'PRAIND';
}
