import { TestBed } from '@angular/core/testing';
import { MatDialog } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { of } from 'rxjs';
import { Avisos } from '../compartido/avisos';
import { CorreoPagina } from './correo-pagina';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const persona = (id: string, nombre: string, via: string) => ({
  via,
  ingreso_id: id,
  identidad: id,
  nombre,
  procedencia: 'Reunión con compras',
  medio: 'A_PIE',
  placa: null,
  gafete: 4,
  sin_gafete: false,
  entrada: '2026-10-09T14:00:00Z',
  entrada_por: { id: 'o1', nombre: 'OPERADOR UNO' },
  demasiado_tiempo: false,
});

const llamadas = (comando: string) =>
  vi.mocked(invoke).mock.calls.filter(([nombre]) => nombre === comando);

async function crear() {
  const fixture = TestBed.createComponent(CorreoPagina);
  fixture.detectChanges();
  await new Promise((resolver) => setTimeout(resolver, 0));
  await fixture.whenStable();
  fixture.detectChanges();
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

describe('pantalla de correo', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue([
      persona('1', 'ANA', 'CONTRATISTA'),
      persona('2', 'BETO', 'CORREO'),
      persona('3', 'CARLA', 'CORREO'),
    ]);
    document.querySelectorAll('.cdk-overlay-container > *').forEach((e) => e.remove());
  });

  it('muestra sólo las visitas por correo que están adentro', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h1')?.textContent).toBe('Correo');
    await vi.waitFor(() =>
      expect(raiz.querySelector('.conteo')?.textContent?.trim()).toBe('2 registros'),
    );
  });

  it('el botón "Registrar ingreso" abre el modal de correo', async () => {
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.ingreso')?.click();
    await fixture.whenStable();

    const modal = document.querySelector('app-dialogo-ingreso-correo');
    expect(modal?.querySelector('h2')?.textContent).toContain('ingreso por correo');
    expect(TestBed.inject(MatDialog).openDialogs[0]?.disableClose).toBe(true);
    TestBed.inject(MatDialog).closeAll();
  });

  it('al registrar un ingreso avisa y vuelve a pedir quién está adentro', async () => {
    TestBed.overrideProvider(MatDialog, {
      useValue: { open: () => ({ afterClosed: () => of({ nombre: 'ANA SOLANO' }) }) },
    });
    const exito = vi.spyOn(TestBed.inject(Avisos), 'exito');
    const { raiz } = await crear();
    const antes = llamadas('dentro').length;

    raiz.querySelector<HTMLButtonElement>('button.ingreso')?.click();
    await vi.waitFor(() => expect(llamadas('dentro').length).toBe(antes + 1));

    expect(exito).toHaveBeenCalledWith('ANA SOLANO', 'Ingreso registrado');
  });
});
