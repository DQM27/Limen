/** Lo que responde el Worker del panel (`apps/panel/src`). Aquí no se decide nada. */

export type TipoDispositivo = 'pc' | 'celular' | 'visor';

export interface Dispositivo {
  id: string;
  nombre: string;
  unidad: string;
  tipo: TipoDispositivo;
  creado: string;
  vinculado: string | null;
  retirado: string | null;
  estado: 'esperando_codigo' | 'vinculado' | 'retirado';
}

export interface EquipoConectado {
  equipo: string;
  unidad: string;
  iniciada: string | null;
  visto: string;
  estado: 'en_linea' | 'caido' | 'fuera';
}

export interface Unidad {
  codigo: string;
  nombre: string;
}

export interface CodigoEmitido {
  id: string;
  codigo: string;
  vigencia_minutos: number;
}

export interface NuevoDispositivo {
  nombre: string;
  unidad: string;
  tipo: TipoDispositivo;
}

/** Un problema de un campo del formulario, tal como lo dice el núcleo (`campo` es el del formulario). */
export interface Problema {
  codigo: string;
  campo: string;
  mensaje: string;
}

/** Un fallo con el código que dio el Worker (`codigo_invalido`, `nube`…), y los problemas del formulario si los hubo. */
export class ErrorPanel extends Error {
  constructor(
    readonly codigo: string,
    readonly problemas: readonly Problema[] = [],
  ) {
    super(codigo);
  }
}

async function pedir<T>(camino: string, metodo = 'GET', cuerpo?: unknown): Promise<T> {
  const respuesta = await fetch(camino, {
    method: metodo,
    headers: cuerpo === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: cuerpo === undefined ? undefined : JSON.stringify(cuerpo),
  });
  const datos = (await respuesta.json().catch(() => ({}))) as {
    error?: string;
    problemas?: Problema[];
  } & T;
  if (!respuesta.ok) {
    throw new ErrorPanel(datos.error ?? (datos.problemas ? 'problemas' : 'nube'), datos.problemas);
  }
  return datos;
}

export const listarDispositivos = async (): Promise<Dispositivo[]> =>
  (await pedir<{ dispositivos: Dispositivo[] }>('/api/dispositivos')).dispositivos;

export const listarUnidades = async (): Promise<Unidad[]> =>
  (await pedir<{ unidades: Unidad[] }>('/api/unidades')).unidades;

export const listarEquipos = async (): Promise<EquipoConectado[]> =>
  (await pedir<{ equipos: EquipoConectado[] }>('/api/equipos')).equipos;

export const registrarDispositivo = (nuevo: NuevoDispositivo): Promise<CodigoEmitido> =>
  pedir<CodigoEmitido>('/api/dispositivos', 'POST', nuevo);

export const retirarDispositivo = async (id: string): Promise<void> => {
  await pedir<unknown>(`/api/dispositivos/${id}/retirar`, 'POST');
};

export interface Usuario {
  id: string;
  cedula: string;
  nombre: string;
  activo: boolean;
  debe_cambiar_clave: boolean;
}

/** Lo que el Worker responde al crear un usuario o restablecer su clave: la clave temporal se ve una sola vez. */
export interface UsuarioCreado {
  usuario: Usuario;
  clave_temporal: string;
}

export const listarUsuarios = async (): Promise<Usuario[]> =>
  (await pedir<{ usuarios: Usuario[] }>('/api/usuarios')).usuarios;

export const crearUsuario = (datos: { cedula: string; nombre: string }): Promise<UsuarioCreado> =>
  pedir<UsuarioCreado>('/api/usuarios', 'POST', datos);

export const editarUsuario = async (
  id: string,
  datos: { cedula: string; nombre: string; activo: boolean },
): Promise<Usuario> =>
  (await pedir<{ usuario: Usuario }>(`/api/usuarios/${id}`, 'PATCH', datos)).usuario;

export const restablecerClaveDeUsuario = async (id: string): Promise<string> =>
  (await pedir<{ clave_temporal: string }>(`/api/usuarios/${id}/clave`, 'POST')).clave_temporal;

export type TipoIngreso = 'PRAIND' | 'IN_HOUSE';

/** Un contratista como lo lista el Worker; la fecha es `AAAA-MM-DD`. */
export interface Contratista {
  id: string;
  cedula: string;
  nombre: string;
  empresa_id: string;
  empresa_nombre: string;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string;
  tiene_acceso: boolean;
}

export interface EmpresaCatalogo {
  id: string;
  nombre: string;
}

/** Lo que se escribe en el formulario del contratista, tal cual: el núcleo lo valida. */
export interface FormularioContratista {
  cedula: string;
  nombre: string;
  empresa_id: string;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string;
  tiene_acceso: boolean;
}

export const listarContratistas = (): Promise<{
  contratistas: Contratista[];
  empresas: EmpresaCatalogo[];
}> => pedir('/api/contratistas');

export const registrarContratista = async (datos: FormularioContratista): Promise<Contratista> =>
  (await pedir<{ contratista: Contratista }>('/api/contratistas', 'POST', datos)).contratista;

export const editarContratista = async (
  id: string,
  datos: FormularioContratista,
): Promise<Contratista> =>
  (await pedir<{ contratista: Contratista }>(`/api/contratistas/${id}`, 'PATCH', datos))
    .contratista;

export const crearEmpresa = async (nombre: string): Promise<EmpresaCatalogo> =>
  (await pedir<{ empresa: EmpresaCatalogo }>('/api/empresas', 'POST', { nombre })).empresa;
