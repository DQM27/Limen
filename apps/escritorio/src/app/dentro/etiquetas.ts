import type { PersonaAdentro, Via } from '../nucleo/tipos';

/**
 * Textos para mostrar lo que dice el núcleo sobre quién está adentro. Sólo
 * traducen: quién está adentro, cuánto lleva y qué le corresponde lo decide
 * el núcleo.
 */

const VIAS: Record<Via, string> = {
  CONTRATISTA: 'Contratista',
  PROVEEDOR: 'Proveedor',
  CORREO: 'Correo',
  KOF: 'KOF',
};

export function etiquetaDeVia(via: Via): string {
  return VIAS[via];
}

/** «A pie», «Vehículo ABC-123» o vacío si no se registra (el personal KOF). */
export function textoMedio(persona: Pick<PersonaAdentro, 'medio' | 'placa'>): string {
  switch (persona.medio) {
    case 'A_PIE':
      return 'A pie';
    case 'VEHICULO':
      return persona.placa ? `Vehículo ${persona.placa}` : 'Vehículo';
    case null:
      return '';
  }
}

/** El número de gafete, «S/G» si entró sin gafete, o «—» si no le aplica. */
export function textoGafete(persona: Pick<PersonaAdentro, 'gafete' | 'sin_gafete'>): string {
  if (persona.gafete !== null) {
    return String(persona.gafete);
  }
  return persona.sin_gafete ? 'S/G' : '—';
}

/** Quién registró la entrada: su nombre o, si ese usuario no está en el equipo, su ID. */
export function textoOperador(operador: PersonaAdentro['entrada_por']): string {
  return operador.nombre ?? operador.id;
}

/** Los filtros rápidos por gafete. */
export type FiltroGafete = 'todos' | 'con' | 'sin';

export function filtrarPorGafete(
  personas: readonly PersonaAdentro[],
  filtro: FiltroGafete,
): PersonaAdentro[] {
  switch (filtro) {
    case 'todos':
      return [...personas];
    case 'con':
      return personas.filter((p) => p.gafete !== null);
    case 'sin':
      return personas.filter((p) => p.gafete === null);
  }
}
