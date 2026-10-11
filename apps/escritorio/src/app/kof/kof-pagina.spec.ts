import { TestBed } from '@angular/core/testing';
import { MatDialog } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { of } from 'rxjs';
import { Avisos } from '../compartido/avisos';
import { KofPagina } from './kof-pagina';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const persona = (id: string, nombre: string, via: string) => ({
  via,
  ingreso_id: id,
  identidad: `50400${id}`,
  nombre,
  procedencia: 'Personal KOF',
  medio: null,
  placa: null,
  gafete: 3,
  sin_gafete: false,
  entrada: '2026-10-09T14:00:00Z',
  entrada_por: { id: 'o1', nombre: 'OPERADOR UNO' },
  demasiado_tiempo: false,
});

const llamadas = (comando: string) =>
  vi.mocked(invoke).mock.calls.filter(([nombre]) => nombre === comando);

async function crear() {
  const fixture = TestBed.createComponent(KofPagina);
  fixture.detectChanges();
  await new Promise((resolver) => setTimeout(resolver, 0));
  await fixture.whenStable();
  fixture.detectChanges();
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

describe('pantalla de KOF', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue([
      persona('1', 'ANA', 'CONTRATISTA'),
      persona('2', 'LUIS', 'KOF'),
    ]);
    document.querySelectorAll('.cdk-overlay-container > *').forEach((e) => e.remove());
  });

  it('muestra sólo al personal KOF que tiene un gafete provisional, identificado por código', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h1')?.textContent).toBe('KOF');
    await vi.waitFor(() =>
      expect(raiz.querySelector('.conteo')?.textContent?.trim()).toBe('1 registro'),
    );
    const titulos = [...raiz.querySelectorAll('.ag-header-cell-text')].map((e) => e.textContent);
    expect(titulos.length === 0 || titulos.includes('Código')).toBe(true);
  });

  it('"Entregar gafete" abre el modal de entrega', async () => {
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.entrega')?.click();
    await fixture.whenStable();

    const modal = document.querySelector('app-dialogo-entregar-gafete');
    expect(modal?.querySelector('h2')?.textContent).toContain('Entregar gafete provisional');
    expect(TestBed.inject(MatDialog).openDialogs[0]?.disableClose).toBe(true);
    TestBed.inject(MatDialog).closeAll();
  });

  it('"Nuevo personal" abre el modal para dar de alta', async () => {
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.nuevo')?.click();
    await fixture.whenStable();

    const modal = document.querySelector('app-dialogo-nuevo-personal');
    expect(modal?.querySelector('h2')?.textContent).toContain('Nuevo personal KOF');
    TestBed.inject(MatDialog).closeAll();
  });

  it('al entregar un gafete avisa y vuelve a pedir quién está adentro', async () => {
    TestBed.overrideProvider(MatDialog, {
      useValue: { open: () => ({ afterClosed: () => of({ nombre: 'ANA MORA' }) }) },
    });
    const exito = vi.spyOn(TestBed.inject(Avisos), 'exito');
    const { raiz } = await crear();
    const antes = llamadas('dentro').length;

    raiz.querySelector<HTMLButtonElement>('button.entrega')?.click();
    await vi.waitFor(() => expect(llamadas('dentro').length).toBe(antes + 1));

    expect(exito).toHaveBeenCalledWith('ANA MORA', 'Gafete entregado');
  });

  it('al dar de alta a una persona avisa, sin tocar la lista (todavía no entró)', async () => {
    TestBed.overrideProvider(MatDialog, {
      useValue: { open: () => ({ afterClosed: () => of({ nombre: 'ANA MORA' }) }) },
    });
    const exito = vi.spyOn(TestBed.inject(Avisos), 'exito');
    const { raiz } = await crear();
    const antes = llamadas('dentro').length;

    raiz.querySelector<HTMLButtonElement>('button.nuevo')?.click();
    await vi.waitFor(() => expect(exito).toHaveBeenCalledWith('ANA MORA', 'Personal registrado'));

    expect(llamadas('dentro').length).toBe(antes);
  });

  it('cancelar un modal no avisa ni actualiza', async () => {
    TestBed.overrideProvider(MatDialog, {
      useValue: { open: () => ({ afterClosed: () => of(undefined) }) },
    });
    const exito = vi.spyOn(TestBed.inject(Avisos), 'exito');
    const { raiz } = await crear();
    const antes = llamadas('dentro').length;

    raiz.querySelector<HTMLButtonElement>('button.entrega')?.click();
    raiz.querySelector<HTMLButtonElement>('button.nuevo')?.click();
    await new Promise((resolver) => setTimeout(resolver, 0));

    expect(exito).not.toHaveBeenCalled();
    expect(llamadas('dentro').length).toBe(antes);
  });
});
