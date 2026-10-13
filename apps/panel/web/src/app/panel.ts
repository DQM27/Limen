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

export const listarEquipos = async (): Promise<EquipoConectado[]> =>
  (await pedir<{ equipos: EquipoConectado[] }>('/api/equipos')).equipos;

export const registrarDispositivo = (nuevo: NuevoDispositivo): Promise<CodigoEmitido> =>
  pedir<CodigoEmitido>('/api/dispositivos', 'POST', nuevo);

export const retirarDispositivo = async (id: string): Promise<void> => {
  await pedir<unknown>(`/api/dispositivos/${id}/retirar`, 'POST');
};
