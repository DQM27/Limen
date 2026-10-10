import { invoke } from '@tauri-apps/api/core';
import {
  buscarEmpresas,
  buscarEmpresasProveedoras,
  buscarPersonalKof,
  cambiarGafete,
  editarContratista,
  editarPersonalKof,
  entregarGafeteKof,
  listarContratistas,
  listarGafetes,
  registrarContratista,
  registrarEmpresa,
  registrarEmpresaProveedora,
  registrarEntradaCorreo,
  registrarEntradaProveedor,
  registrarGafetes,
  registrarPersonalKof,
  renombrarEmpresa,
  renombrarEmpresaProveedora,
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

  it('cada comando de catálogos e ingresos lleva su nombre y sus parámetros exactos', async () => {
    const proveedor = {
      cedula: '2',
      nombre: 'B',
      empresa_id: 'p1',
      medio: 'A_PIE' as const,
      placa: null,
      gafete: 3,
    };
    const correo = { ...proveedor, motivo: 'RH' };
    const perdido = {
      cambio: 'PERDIDO' as const,
      deudor_contratista_id: null,
      deudor_cedula: '333333333',
    };
    const casos: [() => Promise<unknown>, string, Record<string, unknown>][] = [
      [() => renombrarEmpresa('e1', 'ACME'), 'renombrar_empresa', { id: 'e1', nombre: 'ACME' }],
      [
        () => buscarEmpresasProveedoras('gas', 5),
        'buscar_empresas_proveedoras',
        { texto: 'gas', limite: 5 },
      ],
      [() => registrarEmpresaProveedora('GAS'), 'registrar_empresa_proveedora', { nombre: 'GAS' }],
      [
        () => renombrarEmpresaProveedora('p1', 'GAS S.A.'),
        'renombrar_empresa_proveedora',
        { id: 'p1', nombre: 'GAS S.A.' },
      ],
      [
        () => registrarEntradaProveedor(proveedor),
        'registrar_entrada_proveedor',
        { entrada: proveedor },
      ],
      [() => registrarEntradaCorreo(correo), 'registrar_entrada_correo', { entrada: correo }],
      [() => buscarPersonalKof('504', 5), 'buscar_personal_kof', { texto: '504', limite: 5 }],
      // Tauri pasa los parámetros a camelCase: `codigo_empleado` es `codigoEmpleado`.
      [
        () => registrarPersonalKof('5040017', 'ANA'),
        'registrar_personal_kof',
        { codigoEmpleado: '5040017', nombre: 'ANA' },
      ],
      [
        () => editarPersonalKof('k1', 'ANA', false),
        'editar_personal_kof',
        { id: 'k1', nombre: 'ANA', activo: false },
      ],
      [() => entregarGafeteKof('k1', 3), 'entregar_gafete_kof', { personalId: 'k1', gafete: 3 }],
      [() => listarGafetes('VISITA'), 'listar_gafetes', { tipo: 'VISITA' }],
      [
        () => registrarGafetes('VISITA', 1, 10),
        'registrar_gafetes',
        { tipo: 'VISITA', desde: 1, hasta: 10 },
      ],
      [
        () => cambiarGafete('VISITA', 1, perdido),
        'cambiar_gafete',
        { tipo: 'VISITA', numero: 1, cambio: perdido },
      ],
    ];
    for (const [llamar, comando, argumentos] of casos) {
      vi.mocked(invoke).mockClear();
      await llamar();
      expect(invoke).toHaveBeenCalledWith(comando, argumentos);
    }
  });
});
