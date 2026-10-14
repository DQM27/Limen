import type { ICellRendererParams } from 'ag-grid-community';
import type { CampoResaltable, Nucleo } from '../nucleo/nucleo';
import type { ContextoDeTabla } from './tabla';

/**
 * Una celda de texto con lo que coincide con el buscador resaltado (regla N1). La celda no
 * decide qué se resalta: le pregunta al núcleo y pinta lo que responde.
 */
export function celdaResaltada<T>(
  nucleo: Nucleo,
  campo: CampoResaltable,
): (parametros: ICellRendererParams<T>) => HTMLElement {
  return (parametros) => {
    const texto = String(parametros.value ?? '');
    const buscado = (parametros.context as ContextoDeTabla).buscado();
    const contenedor = document.createElement('span');
    if (buscado.trim() === '' || texto === '') {
      contenedor.textContent = texto;
      return contenedor;
    }
    for (const segmento of nucleo.resaltar(buscado, campo, [texto])[0] ?? []) {
      if (segmento.resaltado) {
        const marca = document.createElement('mark');
        marca.textContent = segmento.texto;
        contenedor.append(marca);
      } else {
        contenedor.append(document.createTextNode(segmento.texto));
      }
    }
    return contenedor;
  };
}
