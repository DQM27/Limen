import { invoke } from '@tauri-apps/api/core';
import {
  buscarEmpresas,
  editarContratista,
  listarContratistas,
  registrarContratista,
  registrarEmpresa,
} from './comandos';
import type { ContratistaEntrada } from './tipos';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

const formulario: ContratistaEntrada = {
  cedula: '1-1111-1111',
  nombre: 'josé peña',
  empresa_id: 'e1',
  tipo_ingreso: 'PRAIND',
  fecha_vencimiento_praind: '2099-01-01',
  tiene_acceso: true,
};

/**
 * Cada función llama al comando de Tauri con su nombre y sus parámetros
 * exactos: si uno cambia en Rust, esto tiene que cambiar con él.
 */
describe('comandos del núcleo', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue('ok');
  });

  it('listarContratistas no lleva parámetros', async () => {
    await listarContratistas();
    expect(invoke).toHaveBeenCalledWith('listar_contratistas', undefined);
  });

  it('buscarEmpresas pasa el texto y el límite', async () => {
    await buscarEmpresas('acm', 10);
    expect(invoke).toHaveBeenCalledWith('buscar_empresas', { texto: 'acm', limite: 10 });
  });

  it('registrarEmpresa pasa el nombre tal cual', async () => {
    await registrarEmpresa('acme s.a.');
    expect(invoke).toHaveBeenCalledWith('registrar_empresa', { nombre: 'acme s.a.' });
  });

  it('registrarContratista manda el formulario sin tocarlo', async () => {
    await expect(registrarContratista(formulario)).resolves.toBe('ok');
    expect(invoke).toHaveBeenCalledWith('registrar_contratista', { contratista: formulario });
  });

  it('editarContratista manda el id y el formulario completo', async () => {
    vi.mocked(invoke).mockResolvedValue([{ campo: 'nombre', antes: 'A', despues: 'B' }]);
    await expect(editarContratista('c1', formulario)).resolves.toEqual([
      { campo: 'nombre', antes: 'A', despues: 'B' },
    ]);
    expect(invoke).toHaveBeenCalledWith('editar_contratista', {
      id: 'c1',
      contratista: formulario,
    });
  });

  it('el error del núcleo llega con su campo, sin tocarlo', async () => {
    const error = {
      tipo: 'negocio',
      codigo: 'cedula_repetida',
      mensaje: 'Ya existe un contratista con esa cédula',
      campo: 'cedula',
    };
    vi.mocked(invoke).mockRejectedValue(error);
    await expect(registrarContratista(formulario)).rejects.toBe(error);
  });
});
