import type { Acceso, TipoIngreso } from '../nucleo/tipos';

/**
 * Textos para mostrar los códigos del núcleo. Sólo traducen: qué está vigente,
 * por vencer o vencido lo decidió el dominio y llega ya en `acceso`.
 */

export function etiquetaDeTipo(tipo: TipoIngreso): string {
  return tipo === 'IN_HOUSE' ? 'IN HOUSE' : 'PRAIND';
}

export function etiquetaDeAcceso(acceso: Acceso): string {
  switch (acceso.resultado) {
    case 'PERMITIDO':
      return 'Vigente';
    case 'PERMITIDO_CON_ADVERTENCIA':
      return acceso.dias_para_vencer === 0
        ? 'Vence hoy'
        : `Por vencer (${acceso.dias_para_vencer} días)`;
    case 'DENEGADO':
      return acceso.motivo === 'praind_vencido' ? 'PRAIND vencido' : 'Sin acceso';
  }
}
