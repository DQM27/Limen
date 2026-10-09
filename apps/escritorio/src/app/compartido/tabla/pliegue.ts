import type { TextMatcherParams } from 'ag-grid-community';

/**
 * Cómo busca la tabla: sin importar mayúsculas ni tildes ("Sanchez" encuentra a
 * "Sánchez"; la Ñ cuenta como N, igual que el buscador del núcleo), y por
 * palabras en cualquier orden y todas obligatorias ("carlos sa" encuentra a
 * "Carlos Mauricio Sánchez").
 *
 * Es sólo el filtro de lo que ya está en pantalla; el buscador del núcleo
 * (`dominio/busqueda.rs`) sigue siendo el que busca en la base.
 */

/** Sin tildes y en mayúsculas. */
export function plegar(texto: string): string {
  return texto.normalize('NFD').replace(/[̀-ͯ]/g, '').toUpperCase();
}

/** `quickFilterParser` de AG Grid: cada palabra tecleada, ya plegada. */
export function analizarBusqueda(texto: string): string[] {
  return plegar(texto)
    .split(' ')
    .filter((parte) => parte.length > 0);
}

/** `quickFilterMatcher` de AG Grid: todas las palabras aparecen en la fila. */
export function coincideBusqueda(partes: string[], textoFila: string): boolean {
  const plegado = plegar(textoFila);
  return partes.every((parte) => plegado.includes(parte));
}

/**
 * `textMatcher` del filtro de texto de cada columna. Sólo "contiene" y "no
 * contiene" parten por palabras; las demás opciones (igual, empieza con…)
 * comparan una sola frase.
 */
export function coincideFiltroDeColumna({
  filterOption,
  value,
  filterText,
}: TextMatcherParams): boolean {
  if (filterText == null) {
    return true;
  }
  const valor = plegar(String(value ?? ''));
  const texto = plegar(filterText);
  switch (filterOption) {
    case 'notContains':
      return !valor.includes(texto);
    case 'equals':
      return valor === texto;
    case 'notEqual':
      return valor !== texto;
    case 'startsWith':
      return valor.startsWith(texto);
    case 'endsWith':
      return valor.endsWith(texto);
    default:
      return analizarBusqueda(filterText).every((parte) => valor.includes(parte));
  }
}
