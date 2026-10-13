import { celdasQueCumplen, filasQueCoinciden, resaltarCoincidencias } from './comandos';
import type { CampoDeTexto, CampoResaltable, PedidoDeFiltro, Segmento } from './tipos';

/**
 * Las reglas de búsqueda de las tablas. Viven en el núcleo (reglas N1 y N2,
 * `dominio/resaltado.rs` y `dominio/filtro_de_tabla.rs`): la pantalla no
 * compara, no pliega ni parte por palabras, sólo pregunta y aplica lo que
 * responde. Las tablas reciben esto por parámetro para poder probarse sin
 * núcleo.
 */
export interface ReglasDeBusqueda {
  /** Cada texto partido en segmentos, con lo que coincide resaltado (N1). */
  resaltar(buscado: string, campo: CampoResaltable, textos: string[]): Promise<Segmento[][]>;
  /** De cada fila, si coincide con el buscador de la tabla (N2). */
  filasQueCoinciden(buscado: string, filas: CampoDeTexto[][]): Promise<boolean[]>;
  /** De cada celda, si cumple el filtro de columna que se le pide (N2). */
  celdasQueCumplen(pedidos: PedidoDeFiltro[]): Promise<boolean[]>;
}

/** Las reglas de verdad: las del núcleo. */
export const reglasDelNucleo: ReglasDeBusqueda = {
  resaltar: resaltarCoincidencias,
  filasQueCoinciden,
  celdasQueCumplen,
};
