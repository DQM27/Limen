import { TestBed } from '@angular/core/testing';
import { MatDialog } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import type { Gafete } from '../nucleo/tipos';
import { GafetesPagina } from './gafetes-pagina';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const gafete = (numero: number, cambios: Partial<Gafete> = {}): Gafete => ({
  tipo: 'CONTRATISTA',
  numero,
  estado: 'DISPONIBLE',
  prestado: false,
  portador_contratista_id: null,
  portador_cedula: null,
  portador_personal_id: null,
  asignado_nombre: null,
  asignado_identidad: null,
  ...cambios,
});

async function crear() {
  const fixture = TestBed.createComponent(GafetesPagina);
  await fixture.whenStable();
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

/** Lo que la pantalla expone para probarla sin simular el ratón de la grilla. */
interface Interno {
  cambiarTipo(tipo: string): void;
  gestionar(gafete: Gafete): void;
}

describe('pantalla de gafetes', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('pide los gafetes de contratista y muestra cuántos hay', async () => {
    vi.mocked(invoke).mockResolvedValue([gafete(1), gafete(2), gafete(3)]);

    const { raiz } = await crear();

    expect(invoke).toHaveBeenCalledWith('listar_gafetes', { tipo: 'CONTRATISTA' });
    expect(raiz.querySelector('h1')?.textContent).toBe('Gafetes');
    expect(raiz.querySelector('.conteo')?.textContent?.trim()).toBe('3 registros');
  });

  it('ofrece los cuatro tipos y empieza en contratista', async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    const { raiz } = await crear();

    const tipos = [...raiz.querySelectorAll('mat-button-toggle')].map((t) => t.textContent?.trim());
    expect(tipos).toEqual(['Contratista', 'Visita', 'Proveedor', 'KOF provisional']);
    expect(
      raiz.querySelector('mat-button-toggle.mat-button-toggle-checked')?.textContent?.trim(),
    ).toBe('Contratista');
  });

  it('al cambiar de tipo pide los de ese tipo, sin mezclarlos', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { fixture } = await crear();

    (fixture.componentInstance as unknown as Interno).cambiarTipo('VISITA');
    await fixture.whenStable();

    expect(invoke).toHaveBeenLastCalledWith('listar_gafetes', { tipo: 'VISITA' });
  });

  it('si el núcleo falla, muestra su mensaje tal cual', async () => {
    vi.mocked(invoke).mockRejectedValue({
      tipo: 'negocio',
      codigo: 'sin_sesion',
      mensaje: 'Inicie sesión para continuar',
    });

    const { raiz } = await crear();

    expect(raiz.querySelector('[role="alert"]')?.textContent).toContain(
      'Inicie sesión para continuar',
    );
  });

  it('"Nuevos gafetes" abre el modal del tipo elegido', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.nuevo')?.click();
    await fixture.whenStable();

    const modal = document.querySelector('app-dialogo-crear-gafetes');
    expect(modal?.querySelector('h2')?.textContent).toContain('Nuevos gafetes');
    expect(modal?.querySelector('.tipo')?.textContent).toContain('Contratista');
    expect(TestBed.inject(MatDialog).openDialogs[0]?.disableClose).toBe(true);
    TestBed.inject(MatDialog).closeAll();
  });

  it('gestionar un gafete abre su modal con sus datos', async () => {
    vi.mocked(invoke).mockResolvedValue([gafete(25)]);
    const { fixture } = await crear();

    (fixture.componentInstance as unknown as Interno).gestionar(gafete(25));
    await fixture.whenStable();

    const modal = document.querySelector('app-dialogo-gestion-gafete');
    expect(modal?.querySelector('h2')?.textContent).toContain('Gafete 25');
    TestBed.inject(MatDialog).closeAll();
  });
});
