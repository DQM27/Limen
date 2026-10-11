import { TestBed } from '@angular/core/testing';
import { MatDialog } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import type { FilaContratista } from '../nucleo/tipos';
import { ContratistasPagina } from './contratistas-pagina';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const fila = (id: string, nombre: string): FilaContratista => ({
  id,
  cedula: '111111111',
  nombre,
  empresa_id: 'e1',
  empresa_nombre: 'ACME S.A.',
  tipo_ingreso: 'PRAIND',
  fecha_vencimiento_praind: '2099-01-01',
  tiene_acceso: true,
  requiere_gafete: true,
  acceso: { resultado: 'PERMITIDO', dias_para_vencer: null, motivo: null },
});

async function crear() {
  const fixture = TestBed.createComponent(ContratistasPagina);
  await fixture.whenStable();
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

describe('pantalla de contratistas', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('pide el listado al núcleo y muestra cuántos hay', async () => {
    vi.mocked(invoke).mockResolvedValue([fila('1', 'ANA'), fila('2', 'BETO')]);

    const { raiz } = await crear();

    expect(invoke).toHaveBeenCalledWith('listar_contratistas', undefined);
    expect(raiz.querySelector('h1')?.textContent).toBe('Contratistas');
    expect(raiz.querySelector('.conteo')?.textContent?.trim()).toBe('2 registros');
    expect(raiz.querySelector('[role="alert"]')).toBeNull();
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

  it('ante un fallo que no es del núcleo, muestra un mensaje genérico', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('boom'));

    const { raiz } = await crear();

    expect(raiz.querySelector('[role="alert"]')?.textContent).toContain(
      'No se pudieron cargar los contratistas',
    );
  });

  it('el botón "Nuevo contratista" abre el formulario vacío', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.nuevo')?.click();
    await fixture.whenStable();

    const dialogo = document.querySelector('app-formulario-contratista');
    expect(dialogo?.querySelector('h2')?.textContent).toContain('Nuevo contratista');
    // Un clic fuera no lo cierra.
    expect(TestBed.inject(MatDialog).openDialogs[0]?.disableClose).toBe(true);
    TestBed.inject(MatDialog).closeAll();
  });

  it('abrir una fila para editar precarga sus datos', async () => {
    vi.mocked(invoke).mockResolvedValue([fila('1', 'ANA')]);
    const { fixture } = await crear();

    (
      fixture.componentInstance as unknown as { abrirFormulario(f: FilaContratista): void }
    ).abrirFormulario(fila('1', 'ANA'));
    await fixture.whenStable();

    const dialogo = document.querySelector('app-formulario-contratista');
    expect(dialogo?.querySelector('h2')?.textContent).toContain('Editar contratista');
    expect(dialogo?.querySelector<HTMLInputElement>('input[formControlName=nombre]')?.value).toBe(
      'ANA',
    );
    TestBed.inject(MatDialog).closeAll();
  });

  it('el botón "Registrar ingreso" abre el modal empezando por la búsqueda', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { fixture, raiz } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.ingreso')?.click();
    await fixture.whenStable();

    const modal = document.querySelector('app-dialogo-ingreso');
    expect(modal?.querySelector('h2')?.textContent).toContain('Registrar ingreso');
    expect(modal?.querySelector('mat-label')?.textContent).toContain('Cédula o nombre');
    expect(TestBed.inject(MatDialog).openDialogs[0]?.disableClose).toBe(true);
    TestBed.inject(MatDialog).closeAll();
  });

  it('el botón "Ingreso" de una fila abre el modal directo en esa persona', async () => {
    vi.mocked(invoke).mockImplementation((comando: string) =>
      Promise.resolve(
        comando === 'preparar_ingreso'
          ? { ...fila('1', 'ANA'), puede_entrar: true, gafetes_perdidos: [], adentro_por: null }
          : [fila('1', 'ANA')],
      ),
    );
    const { fixture } = await crear();
    const interno = fixture.componentInstance as unknown as {
      columnas: { colId?: string; onCellClicked?: (e: unknown) => void }[];
    };
    const columna = interno.columnas.find((c) => c.colId === 'accion');
    const boton = document.createElement('button');
    boton.className = 'boton-celda';

    columna?.onCellClicked?.({ data: fila('1', 'ANA'), event: { target: boton } });
    columna?.onCellClicked?.({
      data: fila('2', 'OTRA'),
      event: { target: document.createElement('div') },
    });
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('preparar_ingreso', { contratistaId: '1' });
    expect(invoke).not.toHaveBeenCalledWith('preparar_ingreso', { contratistaId: '2' });
    TestBed.inject(MatDialog).closeAll();
  });

  it('la columna del botón no se ofrece para ocultarla', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { fixture } = await crear();
    const interno = fixture.componentInstance as unknown as {
      columnas: { colId?: string; lockVisible?: boolean }[];
    };

    expect(interno.columnas.find((c) => c.colId === 'accion')?.lockVisible).toBe(true);
  });
});
