import type { Gafete, TipoGafete } from '../nucleo/tipos';

/**
 * Textos para mostrar los códigos del núcleo. Sólo traducen: qué se puede
 * hacer con cada gafete lo decide (y lo exige) el núcleo.
 */

/** Los tipos, en el orden en que se ofrecen. */
export const TIPOS_DE_GAFETE: readonly { tipo: TipoGafete; etiqueta: string }[] = [
  { tipo: 'CONTRATISTA', etiqueta: 'Contratista' },
  { tipo: 'VISITA', etiqueta: 'Visita' },
  { tipo: 'PROVEEDOR', etiqueta: 'Proveedor' },
  { tipo: 'PROVISIONAL_KOF', etiqueta: 'KOF provisional' },
];

export function etiquetaDeTipoGafete(tipo: TipoGafete): string {
  return TIPOS_DE_GAFETE.find((t) => t.tipo === tipo)?.etiqueta ?? tipo;
}

/** Un gafete en manos de alguien ahora se ve «Asignado»; los demás, por su estado. */
export function etiquetaDeEstadoGafete(gafete: Pick<Gafete, 'estado' | 'prestado'>): string {
  if (gafete.prestado) {
    return 'Asignado';
  }
  switch (gafete.estado) {
    case 'DISPONIBLE':
      return 'Disponible';
    case 'PERDIDO':
      return 'Perdido';
    case 'DE_BAJA':
      return 'De baja';
  }
}

/** «ANA SOLANO (111111111)»; sólo la cédula si no hay nombre; vacío si no está asignado. */
export function textoAsignado(
  gafete: Pick<Gafete, 'asignado_nombre' | 'asignado_identidad'>,
): string {
  const { asignado_nombre: nombre, asignado_identidad: identidad } = gafete;
  if (nombre && identidad) {
    return `${nombre} (${identidad})`;
  }
  return nombre ?? identidad ?? '';
}
