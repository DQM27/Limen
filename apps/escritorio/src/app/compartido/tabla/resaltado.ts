import type { ColDef, ICellRendererParams } from 'ag-grid-community';
import type { CampoResaltable, Segmento } from '../../nucleo/tipos';

/**
 * Resaltado de lo que coincide con el buscador (regla N1).
 *
 * La regla vive en el núcleo (`dominio/resaltado.rs`): la pantalla sólo pide
 * los segmentos de cada texto y pinta los resaltados. Nada de comparar,
 * plegar ni contar caracteres aquí.
 *
 * Declarar el campo de una columna con `resaltable()` hace dos cosas: sus
 * celdas se resaltan y el buscador de la tabla la tiene en cuenta (N2).
 */

/** Lo que la tabla pone a disposición de las celdas (el `context` del grid). */
export interface ContextoDeResaltado {
  segmentosDe(campo: CampoResaltable, texto: string): readonly Segmento[] | undefined;
}

/**
 * Cuántos textos distintos por columna se piden al núcleo. Con más filas que
 * esto, las que sobran se muestran sin resaltar: es sólo un apoyo visual.
 */
export const MAXIMO_TEXTOS_POR_COLUMNA = 2000;

interface ParametrosDeCelda {
  campo: CampoResaltable;
}

/** Marca una columna como resaltable: sus celdas pintan lo que coincide. */
export function resaltable<T>(columna: ColDef<T>, campo: CampoResaltable): ColDef<T> {
  const parametros: ParametrosDeCelda = { campo };
  return { ...columna, cellRenderer: pintarCelda, cellRendererParams: parametros };
}

/** La clase de texto de una columna resaltable, o `undefined` si no lo es. */
export function campoResaltable<T>(columna: ColDef<T>): CampoResaltable | undefined {
  return (columna.cellRendererParams as Partial<ParametrosDeCelda> | undefined)?.campo;
}

/** Con qué clave se guardan los segmentos de un texto de cierto campo. */
export function claveDeTexto(campo: CampoResaltable, texto: string): string {
  return `${campo}\u0000${texto}`;
}

/**
 * Pinta una celda. Los resaltados van en `<mark>`; todo entra como texto
 * (nunca como HTML), así que un nombre con símbolos no puede inyectar nada.
 * Sin segmentos (aún no llegaron, o no hay búsqueda) muestra el texto tal cual.
 */
export function pintarCelda(parametros: ICellRendererParams): HTMLElement {
  const celda = document.createElement('span');
  const texto = String(parametros.valueFormatted ?? parametros.value ?? '');
  const campo = (parametros as Partial<ParametrosDeCelda>).campo;
  const contexto = parametros.context as Partial<ContextoDeResaltado> | undefined;
  const segmentos = campo ? contexto?.segmentosDe?.(campo, texto) : undefined;
  if (!segmentos) {
    celda.textContent = texto;
    return celda;
  }
  for (const segmento of segmentos) {
    if (segmento.resaltado) {
      const marca = document.createElement('mark');
      marca.textContent = segmento.texto;
      celda.append(marca);
    } else {
      celda.append(document.createTextNode(segmento.texto));
    }
  }
  return celda;
}
