import type { OperacionDeFiltro, PedidoDeFiltro } from '../../nucleo/tipos';

/**
 * Los filtros de texto de cada columna (regla N2) los decide el núcleo, pero
 * AG Grid pregunta celda por celda y de forma síncrona. Esta clase junta las
 * dos cosas: recuerda lo que el núcleo ya respondió y las celdas que faltan
 * las pide todas juntas, en un solo viaje. No sabe comparar nada.
 *
 * Mientras llega la respuesta, la celda cuenta como que cumple: una fila no
 * desaparece por esperar. Al llegar, se avisa para que la grilla vuelva a
 * filtrar, ya con todo recordado.
 */

/** Cuántos filtros distintos (operación y texto) se recuerdan a la vez. */
const MAXIMO_FILTROS_RECORDADOS = 8;

/** Los filtros de texto de AG Grid y su código en el núcleo. */
const OPERACIONES: Readonly<Record<string, OperacionDeFiltro>> = {
  contains: 'CONTIENE',
  notContains: 'NO_CONTIENE',
  equals: 'IGUAL',
  notEqual: 'DISTINTO',
  startsWith: 'EMPIEZA_CON',
  endsWith: 'TERMINA_CON',
};

/** El código que el núcleo entiende para una opción del filtro de AG Grid. */
export function operacionDeFiltro(opcion: string | null | undefined): OperacionDeFiltro {
  return (opcion && OPERACIONES[opcion]) || 'CONTIENE';
}

interface Lote {
  operacion: OperacionDeFiltro;
  filtro: string;
  celdas: Set<string>;
}

const claveDeFiltro = (operacion: OperacionDeFiltro, filtro: string): string =>
  `${operacion}\u0000${filtro}`;

export class DecisionesDeColumna {
  private readonly decididas = new Map<string, Map<string, boolean>>();
  private pendientes = new Map<string, Lote>();
  private temporizador: ReturnType<typeof setTimeout> | undefined;

  constructor(
    private readonly preguntar: (pedidos: PedidoDeFiltro[]) => Promise<boolean[]>,
    private readonly alDecidir: () => void,
  ) {}

  /** Lo que respondió el núcleo, o `true` mientras se le pregunta. */
  cumple(operacion: OperacionDeFiltro, filtro: string, celda: string): boolean {
    const decision = this.decididas.get(claveDeFiltro(operacion, filtro))?.get(celda);
    if (decision !== undefined) {
      return decision;
    }
    this.pedir(operacion, filtro, celda);
    return true;
  }

  /** Deja de esperar respuestas (la tabla se destruye). */
  cancelar(): void {
    clearTimeout(this.temporizador);
    this.temporizador = undefined;
    this.pendientes = new Map();
  }

  private pedir(operacion: OperacionDeFiltro, filtro: string, celda: string): void {
    const clave = claveDeFiltro(operacion, filtro);
    const lote = this.pendientes.get(clave) ?? { operacion, filtro, celdas: new Set<string>() };
    lote.celdas.add(celda);
    this.pendientes.set(clave, lote);
    // Se espera a que termine de preguntar por todas las filas: un solo viaje.
    this.temporizador ??= setTimeout(() => void this.resolver(), 0);
  }

  private async resolver(): Promise<void> {
    const lotes = [...this.pendientes.values()];
    this.pendientes = new Map();
    this.temporizador = undefined;
    const pedidos: PedidoDeFiltro[] = lotes.flatMap(({ operacion, filtro, celdas }) =>
      [...celdas].map((celda) => ({ operacion, celda, filtro })),
    );
    let respuestas: boolean[];
    try {
      respuestas = await this.preguntar(pedidos);
    } catch {
      // Sin el núcleo no se oculta nada; así tampoco se vuelve a preguntar.
      respuestas = pedidos.map(() => true);
    }
    pedidos.forEach((pedido, i) => this.recordar(pedido, respuestas[i] ?? true));
    this.alDecidir();
  }

  private recordar({ operacion, filtro, celda }: PedidoDeFiltro, decision: boolean): void {
    const clave = claveDeFiltro(operacion, filtro);
    let deEsteFiltro = this.decididas.get(clave);
    if (!deEsteFiltro) {
      if (this.decididas.size >= MAXIMO_FILTROS_RECORDADOS) {
        const masViejo = this.decididas.keys().next().value;
        if (masViejo !== undefined) {
          this.decididas.delete(masViejo);
        }
      }
      deEsteFiltro = new Map();
      this.decididas.set(clave, deEsteFiltro);
    }
    deEsteFiltro.set(celda, decision);
  }
}
