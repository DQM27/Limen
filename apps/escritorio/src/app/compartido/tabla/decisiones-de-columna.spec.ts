import type { PedidoDeFiltro } from '../../nucleo/tipos';
import { DecisionesDeColumna, operacionDeFiltro } from './decisiones-de-columna';

const esperar = (ms = 5): Promise<void> => new Promise((fin) => setTimeout(fin, ms));

function crear(responder: (pedidos: PedidoDeFiltro[]) => Promise<boolean[]>) {
  const preguntar = vi.fn(responder);
  const alDecidir = vi.fn();
  return { decisiones: new DecisionesDeColumna(preguntar, alDecidir), preguntar, alDecidir };
}

describe('decisiones de los filtros de columna (regla N2)', () => {
  it('traduce las opciones del filtro de AG Grid a los códigos del núcleo', () => {
    expect(operacionDeFiltro('contains')).toBe('CONTIENE');
    expect(operacionDeFiltro('notContains')).toBe('NO_CONTIENE');
    expect(operacionDeFiltro('equals')).toBe('IGUAL');
    expect(operacionDeFiltro('notEqual')).toBe('DISTINTO');
    expect(operacionDeFiltro('startsWith')).toBe('EMPIEZA_CON');
    expect(operacionDeFiltro('endsWith')).toBe('TERMINA_CON');
    expect(operacionDeFiltro(undefined), 'sin opción, "contiene"').toBe('CONTIENE');
    expect(operacionDeFiltro('algo raro')).toBe('CONTIENE');
  });

  it('mientras el núcleo no responde, la celda cumple: la fila no desaparece por esperar', () => {
    const { decisiones } = crear(async (pedidos) => pedidos.map(() => false));

    expect(decisiones.cumple('CONTIENE', 'ana', 'BETO')).toBe(true);
  });

  it('junta todas las celdas que faltan en un solo viaje y no repite las ya pedidas', async () => {
    const { decisiones, preguntar } = crear(async (pedidos) =>
      pedidos.map((p) => p.celda === 'ANA'),
    );

    for (const celda of ['ANA', 'BETO', 'ANA', 'CARLA']) {
      decisiones.cumple('CONTIENE', 'ana', celda);
    }
    await esperar();

    expect(preguntar).toHaveBeenCalledTimes(1);
    expect(preguntar).toHaveBeenCalledWith([
      { operacion: 'CONTIENE', celda: 'ANA', filtro: 'ana' },
      { operacion: 'CONTIENE', celda: 'BETO', filtro: 'ana' },
      { operacion: 'CONTIENE', celda: 'CARLA', filtro: 'ana' },
    ]);
  });

  it('recuerda lo que respondió el núcleo y avisa para que la grilla vuelva a filtrar', async () => {
    const { decisiones, preguntar, alDecidir } = crear(async (pedidos) =>
      pedidos.map((p) => p.celda === 'ANA'),
    );
    decisiones.cumple('CONTIENE', 'ana', 'ANA');
    decisiones.cumple('CONTIENE', 'ana', 'BETO');
    await esperar();

    expect(alDecidir).toHaveBeenCalledTimes(1);
    expect(decisiones.cumple('CONTIENE', 'ana', 'ANA')).toBe(true);
    expect(decisiones.cumple('CONTIENE', 'ana', 'BETO')).toBe(false);
    await esperar();
    expect(preguntar, 'ya estaba todo recordado').toHaveBeenCalledTimes(1);
  });

  it('la misma celda con otra operación o con otro texto es otra pregunta', async () => {
    const { decisiones, preguntar } = crear(async (pedidos) => pedidos.map(() => false));
    decisiones.cumple('CONTIENE', 'ana', 'ANA');
    await esperar();

    decisiones.cumple('IGUAL', 'ana', 'ANA');
    decisiones.cumple('CONTIENE', 'beto', 'ANA');
    await esperar();

    expect(preguntar).toHaveBeenCalledTimes(2);
  });

  it('si el núcleo falla, deja pasar todo y no vuelve a preguntar en bucle', async () => {
    const { decisiones, preguntar, alDecidir } = crear(async () => {
      throw new Error('sin núcleo');
    });
    decisiones.cumple('CONTIENE', 'ana', 'BETO');
    await esperar();

    expect(alDecidir).toHaveBeenCalledTimes(1);
    expect(decisiones.cumple('CONTIENE', 'ana', 'BETO')).toBe(true);
    await esperar();
    expect(preguntar).toHaveBeenCalledTimes(1);
  });

  it('sólo recuerda los últimos filtros: la memoria no crece sin límite', async () => {
    const { decisiones, preguntar } = crear(async (pedidos) => pedidos.map(() => false));
    for (let i = 0; i < 10; i++) {
      decisiones.cumple('CONTIENE', `filtro${i}`, 'ANA');
      await esperar();
    }
    expect(preguntar).toHaveBeenCalledTimes(10);

    decisiones.cumple('CONTIENE', 'filtro9', 'ANA');
    await esperar();
    expect(preguntar, 'el más reciente sigue recordado').toHaveBeenCalledTimes(10);

    decisiones.cumple('CONTIENE', 'filtro0', 'ANA');
    await esperar();
    expect(preguntar, 'el más viejo se olvidó y se vuelve a preguntar').toHaveBeenCalledTimes(11);
  });

  it('al cancelar, las preguntas pendientes no se envían', async () => {
    const { decisiones, preguntar } = crear(async (pedidos) => pedidos.map(() => true));
    decisiones.cumple('CONTIENE', 'ana', 'ANA');
    decisiones.cancelar();
    await esperar();

    expect(preguntar).not.toHaveBeenCalled();
  });
});
