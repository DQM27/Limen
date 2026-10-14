import type { Dispositivo, EquipoConectado } from '../panel';

/** Un equipo en la tabla: lo que se registró en el panel y cómo está conectado ahora. */
export interface FilaEquipo {
  id: string;
  nombre: string;
  unidad: string;
  tipo: string;
  /** `sin_registrar`: anunció su conexión pero el panel no lo conoce (por ejemplo, de desarrollo). */
  registro: Dispositivo['estado'] | 'sin_registrar';
  /** `null`: nunca anunció su conexión. */
  conexion: EquipoConectado['estado'] | null;
  visto: string | null;
  creado: string | null;
}

export const ETIQUETA_REGISTRO: Record<FilaEquipo['registro'], string> = {
  esperando_codigo: 'Esperando su código',
  vinculado: 'Vinculado',
  retirado: 'Retirado',
  sin_registrar: 'Sin registrar',
};

export const ETIQUETA_CONEXION: Record<NonNullable<FilaEquipo['conexion']>, string> = {
  en_linea: 'En línea',
  caido: 'Caído',
  fuera: 'Fuera',
};

export const ETIQUETA_TIPO: Record<string, string> = {
  pc: 'PC',
  celular: 'Celular',
  visor: 'Visor',
};

/**
 * Junta los dos listados del Worker en uno solo: cada equipo registrado con su conexión
 * (el ID del equipo es el del registro) y, al final, los que se conectaron sin estar
 * registrados. Sólo ordena y une; qué significa cada estado lo decide el Worker.
 */
export function unirEquipos(
  dispositivos: readonly Dispositivo[],
  conectados: readonly EquipoConectado[],
): FilaEquipo[] {
  const conexionPorId = new Map(conectados.map((c) => [c.equipo, c]));
  const registrados = new Set(dispositivos.map((d) => d.id));

  const porUnidadYNombre = (a: FilaEquipo, b: FilaEquipo): number =>
    a.unidad.localeCompare(b.unidad, 'es') || a.nombre.localeCompare(b.nombre, 'es');

  const filas: FilaEquipo[] = dispositivos.map((d) => {
    const conexion = conexionPorId.get(d.id);
    return {
      id: d.id,
      nombre: d.nombre,
      unidad: d.unidad,
      tipo: d.tipo,
      registro: d.estado,
      conexion: conexion?.estado ?? null,
      visto: conexion?.visto ?? null,
      creado: d.creado,
    };
  });

  const sinRegistrar: FilaEquipo[] = conectados
    .filter((c) => !registrados.has(c.equipo))
    .map((c) => ({
      id: c.equipo,
      nombre: c.equipo.slice(0, 8),
      unidad: c.unidad,
      tipo: '',
      registro: 'sin_registrar',
      conexion: c.estado,
      visto: c.visto,
      creado: null,
    }));

  // Los registrados primero, y al final los que el panel no conoce.
  return [...filas.sort(porUnidadYNombre), ...sinRegistrar.sort(porUnidadYNombre)];
}
