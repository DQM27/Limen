import { invoke } from '@tauri-apps/api/core';
import {
  buscarEmpresas,
  buscarParaIngreso,
  dentro,
  prepararIngreso,
  registrarEntradaContratista,
  registrarSalida,
  registrarSalidaPorGafete,
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
  cambiarClave,
  cerrarSesion,
  crearPrimerUsuario,
  editarUsuario,
  hayUsuarios,
  iniciarSesion,
  listarUsuarios,
  registrarUsuario,
  restablecerClave,
  usuarioActual,
  atajosDeFecha,
  listarHistorial,
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
      portador_contratista_id: null,
      portador_cedula: '333333333',
      portador_personal_id: null,
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

  it('los comandos del ingreso y de las salidas llevan su nombre y sus parámetros exactos', async () => {
    const entrada = {
      contratista_id: 'c1',
      medio: 'A_PIE' as const,
      placa: null,
      gafete: null,
      sin_gafete: true,
    };
    const casos: [() => Promise<unknown>, string, Record<string, unknown> | undefined][] = [
      [() => buscarParaIngreso('ana', 10), 'buscar_para_ingreso', { texto: 'ana', limite: 10 }],
      // Tauri pasa los parámetros a camelCase: `contratista_id` es `contratistaId`.
      [() => prepararIngreso('c1'), 'preparar_ingreso', { contratistaId: 'c1' }],
      [() => registrarEntradaContratista(entrada), 'registrar_entrada_contratista', { entrada }],
      [() => dentro(), 'dentro', undefined],
      [
        () => registrarSalida('CONTRATISTA', 'i1'),
        'registrar_salida',
        { via: 'CONTRATISTA', ingresoId: 'i1' },
      ],
      [
        () => registrarSalidaPorGafete('PROVEEDOR', 4),
        'registrar_salida_por_gafete',
        { via: 'PROVEEDOR', numero: 4 },
      ],
    ];
    for (const [llamar, comando, argumentos] of casos) {
      vi.mocked(invoke).mockClear();
      await llamar();
      expect(invoke).toHaveBeenCalledWith(comando, argumentos);
    }
  });

  it('los comandos de sesión y de usuarios llevan su nombre y sus parámetros exactos', async () => {
    const usuario = { cedula: '1-1111-1111', nombre: 'ana mora', clave: 'portería segura' };
    const casos: [() => Promise<unknown>, string, Record<string, unknown> | undefined][] = [
      [() => hayUsuarios(), 'hay_usuarios', undefined],
      [() => crearPrimerUsuario(usuario), 'crear_primer_usuario', { usuario }],
      [
        () => iniciarSesion('111111111', 'portería segura'),
        'iniciar_sesion',
        { cedula: '111111111', clave: 'portería segura' },
      ],
      [() => cerrarSesion(), 'cerrar_sesion', undefined],
      [() => usuarioActual(), 'usuario_actual', undefined],
      // `clave_actual` en Rust es `claveActual` del lado de JavaScript.
      [
        () => cambiarClave('temporal 123', 'la mía'),
        'cambiar_clave',
        { claveActual: 'temporal 123', clave: 'la mía' },
      ],
      [() => listarUsuarios(), 'listar_usuarios', undefined],
      [() => registrarUsuario(usuario), 'registrar_usuario', { usuario }],
      [
        () => editarUsuario('u1', 'ana mora', false),
        'editar_usuario',
        { id: 'u1', nombre: 'ana mora', activo: false },
      ],
      [
        () => restablecerClave('u1', 'nueva temporal'),
        'restablecer_clave',
        { id: 'u1', clave: 'nueva temporal' },
      ],
    ];
    for (const [llamar, comando, argumentos] of casos) {
      vi.mocked(invoke).mockClear();
      await llamar();
      expect(invoke).toHaveBeenCalledWith(comando, argumentos);
    }
  });

  it('los comandos del historial llevan su nombre y sus parámetros exactos', async () => {
    await listarHistorial('2026-10-01', null);
    expect(invoke).toHaveBeenCalledWith('listar_historial', { desde: '2026-10-01', hasta: null });
    vi.mocked(invoke).mockClear();
    await atajosDeFecha();
    expect(invoke).toHaveBeenCalledWith('atajos_de_fecha', undefined);
  });
});
