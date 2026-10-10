import { TestBed } from '@angular/core/testing';
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
});
