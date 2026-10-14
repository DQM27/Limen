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

/** Un fallo con el código que dio el Worker (`codigo_invalido`, `nube`…). */
export class ErrorPanel extends Error {
  constructor(readonly codigo: string) {
    super(codigo);
  }
}

async function pedir<T>(camino: string, metodo = 'GET', cuerpo?: unknown): Promise<T> {
  const respuesta = await fetch(camino, {
    method: metodo,
    headers: cuerpo === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: cuerpo === undefined ? undefined : JSON.stringify(cuerpo),
  });
  const datos = (await respuesta.json().catch(() => ({}))) as { error?: string } & T;
  if (!respuesta.ok) {
    throw new ErrorPanel(datos.error ?? 'nube');
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
