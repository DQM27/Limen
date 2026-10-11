import { TestBed } from '@angular/core/testing';
import { invoke } from '@tauri-apps/api/core';
import type { AtajoFecha, Historial, Movimiento } from '../nucleo/tipos';
import { HistorialPagina } from './historial-pagina';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const atajo = (
  codigo: string,
  etiqueta: string,
  corta: string,
  por_omision = false,
): AtajoFecha => ({
  codigo,
  etiqueta,
  corta,
  desde: '2026-04-10',
  hasta: '2026-10-10',
  por_omision,
});

const ATAJOS = [
  atajo('HOY', 'Hoy', 'Hoy'),
  atajo('ULTIMOS_6_MESES', 'Últimos 6 meses', '6 meses', true),
];

const movimiento = (id: string, salida: string | null): Movimiento => ({
  via: 'CONTRATISTA',
  ingreso_id: id,
  identidad: '1-1111-1111',
  nombre: `PERSONA ${id}`,
  procedencia: 'ACME',
  medio: 'A_PIE',
  placa: null,
  gafete: 3,
  sin_gafete: false,
  entrada: '2026-10-09T14:00:00Z',
  entrada_por: { id: 'o1', nombre: 'OPERADOR UNO' },
  salida,
  salida_por: salida ? { id: 'o2', nombre: 'OPERADOR DOS' } : null,
});

const historial = (movimientos: Movimiento[], truncado = false): Historial => ({
  desde: '2026-04-10',
  hasta: '2026-10-10',
  movimientos,
  truncado,
  maximo: 5000,
});

const llamadas = (comando: string) =>
  vi.mocked(invoke).mock.calls.filter(([nombre]) => nombre === comando);

function responder(lista: Historial | { codigo: string; mensaje: string; campo: string | null }) {
  vi.mocked(invoke).mockImplementation((comando: string) => {
    if (comando === 'atajos_de_fecha') {
      return Promise.resolve(ATAJOS);
    }
    return 'codigo' in lista
      ? Promise.reject({ tipo: 'negocio', ...lista })
      : Promise.resolve(lista);
  });
}

async function crear() {
  const fixture = TestBed.createComponent(HistorialPagina);
  fixture.detectChanges();
  await new Promise((resolver) => setTimeout(resolver, 0));
  await fixture.whenStable();
  fixture.detectChanges();
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

describe('pantalla de Historial', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder(historial([movimiento('1', '2026-10-09T16:00:00Z'), movimiento('2', null)]));
  });

  it('abre con el rango por omisión que da el núcleo y muestra los movimientos', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h1')?.textContent).toBe('Historial');
    await vi.waitFor(() =>
      expect(raiz.querySelector('.conteo')?.textContent?.trim()).toBe('2 registros'),
    );
    expect(llamadas('listar_historial')[0]?.[1]).toEqual({
      desde: '2026-04-10',
      hasta: '2026-10-10',
    });
    expect(raiz.querySelector('.atajo')?.textContent).toContain('6 meses');
    expect(raiz.querySelector<HTMLInputElement>('input[name="desde"]')?.value).toBe('2026-04-10');
    expect(raiz.querySelector<HTMLInputElement>('input[name="hasta"]')?.value).toBe('2026-10-10');
  });

  it('elegir un acceso rápido cambia el rango y vuelve a consultar', async () => {
    const { fixture, raiz } = await crear();
    const componente = fixture.componentInstance as unknown as {
      elegirAtajo(a: AtajoFecha): void;
    };

    componente.elegirAtajo(ATAJOS[0] as AtajoFecha);
    fixture.detectChanges();
    await vi.waitFor(() => expect(llamadas('listar_historial')).toHaveLength(2));

    expect(raiz.querySelector('.atajo')?.textContent).toContain('Hoy');
  });

  it('escribir una fecha a mano deja el rango como «Personalizado» y consulta de nuevo', async () => {
    const { fixture, raiz } = await crear();
    const desde = raiz.querySelector<HTMLInputElement>('input[name="desde"]');

    if (!desde) {
      throw new Error('no hay campo de fecha');
    }
    desde.value = '2026-10-01';
    desde.dispatchEvent(new Event('change'));
    fixture.detectChanges();
    await vi.waitFor(() => expect(llamadas('listar_historial')).toHaveLength(2));

    expect(llamadas('listar_historial')[1]?.[1]).toEqual({
      desde: '2026-10-01',
      hasta: '2026-10-10',
    });
    expect(raiz.querySelector('.atajo')?.textContent).toContain('Personalizado');
  });

  it('una fecha vacía se manda como sin límite', async () => {
    const { fixture, raiz } = await crear();
    const hasta = raiz.querySelector<HTMLInputElement>('input[name="hasta"]');

    if (!hasta) {
      throw new Error('no hay campo de fecha');
    }
    hasta.value = '';
    hasta.dispatchEvent(new Event('change'));
    fixture.detectChanges();
    await vi.waitFor(() => expect(llamadas('listar_historial')).toHaveLength(2));

    expect(llamadas('listar_historial')[1]?.[1]).toEqual({ desde: '2026-04-10', hasta: null });
  });

  it('si hay más movimientos de los que caben, avisa que se acote el rango', async () => {
    responder(historial([movimiento('1', null)], true));
    const { raiz } = await crear();

    expect(raiz.querySelector('.aviso')?.textContent).toContain('más de 5000');
  });

  it('un rango que el núcleo rechaza se muestra y marca la fecha culpable', async () => {
    responder({ codigo: 'rango_invertido', mensaje: 'La fecha final es anterior', campo: 'hasta' });
    const { raiz } = await crear();

    expect(raiz.querySelector('.error')?.textContent).toContain('La fecha final es anterior');
    expect(raiz.querySelector('input[name="hasta"]')?.getAttribute('aria-invalid')).toBe('true');
    expect(raiz.querySelector('input[name="desde"]')?.getAttribute('aria-invalid')).toBe('false');
  });

  it('«Actualizar» vuelve a consultar el mismo rango', async () => {
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.actualizar')?.click();
    fixture.detectChanges();
    await vi.waitFor(() => expect(llamadas('listar_historial')).toHaveLength(2));

    expect(llamadas('listar_historial')[1]?.[1]).toEqual(llamadas('listar_historial')[0]?.[1]);
  });
});
