import { TestBed } from '@angular/core/testing';
import { Avisos, AvisosPanel } from './avisos';

function crear() {
  const fixture = TestBed.createComponent(AvisosPanel);
  return {
    fixture,
    raiz: fixture.nativeElement as HTMLElement,
    avisos: TestBed.inject(Avisos),
    tarjetas: () => [...fixture.nativeElement.querySelectorAll('article.aviso')] as HTMLElement[],
  };
}

describe('avisos', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('un aviso de éxito muestra su título, su mensaje y su ícono', async () => {
    const { fixture, avisos, tarjetas } = crear();

    avisos.exito('JEFFERSON GARRO', 'Salida registrada');
    fixture.detectChanges();

    expect(tarjetas()).toHaveLength(1);
    const aviso = tarjetas()[0];
    expect(aviso?.dataset['tipo']).toBe('exito');
    expect(aviso?.querySelector('.titulo')?.textContent).toBe('Salida registrada');
    expect(aviso?.querySelector('.mensaje')?.textContent).toBe('JEFFERSON GARRO');
    expect(aviso?.querySelector('.insignia')?.textContent?.trim()).toBe('check_circle');
  });

  it('sin título, usa el que corresponde a su tipo', () => {
    const { fixture, avisos, tarjetas } = crear();

    avisos.exito('a');
    avisos.error('b');
    avisos.info('c');
    fixture.detectChanges();

    // El más nuevo va arriba.
    expect(tarjetas().map((t) => t.querySelector('.titulo')?.textContent)).toEqual([
      'Aviso',
      'No se pudo',
      'Listo',
    ]);
    expect(tarjetas().map((t) => t.dataset['tipo'])).toEqual(['info', 'error', 'exito']);
  });

  it('los errores se anuncian con rol de alerta y el resto como estado', () => {
    const { fixture, avisos, tarjetas } = crear();

    avisos.error('falló');
    avisos.exito('bien');
    fixture.detectChanges();

    expect(tarjetas().map((t) => t.getAttribute('role'))).toEqual(['status', 'alert']);
  });

  it('se va solo: primero se anima la salida y luego se quita', () => {
    const { fixture, avisos, tarjetas } = crear();
    avisos.exito('Listo');
    fixture.detectChanges();

    vi.advanceTimersByTime(4500);
    fixture.detectChanges();
    expect(tarjetas()[0]?.classList).toContain('saliendo');

    vi.advanceTimersByTime(300);
    fixture.detectChanges();
    expect(tarjetas()).toHaveLength(0);
  });

  it('un error se queda más tiempo que un éxito', () => {
    const { fixture, avisos, tarjetas } = crear();
    avisos.error('falló');
    avisos.exito('bien');
    fixture.detectChanges();

    vi.advanceTimersByTime(5000);
    fixture.detectChanges();

    expect(tarjetas().map((t) => t.dataset['tipo'])).toEqual(['error']);
  });

  it('un clic en el aviso o en su cruz lo cierra', () => {
    const { fixture, avisos, tarjetas } = crear();
    avisos.exito('uno');
    avisos.info('dos');
    fixture.detectChanges();

    tarjetas()[0]?.click();
    tarjetas()[1]?.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    vi.advanceTimersByTime(300);
    fixture.detectChanges();

    expect(tarjetas()).toHaveLength(0);
  });

  it('la cruz tiene nombre para los lectores de pantalla', () => {
    const { fixture, avisos, tarjetas } = crear();
    avisos.info('hola');
    fixture.detectChanges();

    expect(tarjetas()[0]?.querySelector('button.cerrar')?.getAttribute('aria-label')).toBe(
      'Cerrar el aviso',
    );
  });

  it('se apilan: llegan hasta cuatro y, con uno más, se va el más viejo', () => {
    const { fixture, avisos, tarjetas } = crear();

    for (const n of [1, 2, 3, 4, 5]) {
      avisos.info(`mensaje ${n}`);
    }
    fixture.detectChanges();

    expect(tarjetas().map((t) => t.querySelector('.mensaje')?.textContent)).toEqual([
      'mensaje 5',
      'mensaje 4',
      'mensaje 3',
      'mensaje 2',
    ]);
  });

  it('cerrar dos veces el mismo aviso no falla', () => {
    const { fixture, avisos, tarjetas } = crear();
    avisos.info('hola');
    fixture.detectChanges();
    const id = avisos.avisos()[0]?.id ?? 0;

    avisos.cerrar(id);
    avisos.cerrar(id);
    vi.advanceTimersByTime(300);
    fixture.detectChanges();

    expect(tarjetas()).toHaveLength(0);
  });
});
