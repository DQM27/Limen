/**
 * Formatos para mostrar. Sólo presentación: ninguna regla de negocio vive
 * aquí (qué está vencido o por vencer lo decide el núcleo).
 */

const ZONA = 'America/Costa_Rica';

/** `2026-10-09` → `09/10/2026`. Un valor vacío se muestra como `—`. */
export function fechaCorta(fecha: string | null | undefined): string {
  if (!fecha) {
    return '—';
  }
  const [anio, mes, dia] = fecha.slice(0, 10).split('-');
  return anio && mes && dia ? `${dia}/${mes}/${anio}` : fecha;
}

/** Un instante UTC (RFC 3339) → `09/10/2026 08:30`, a la hora de Costa Rica. */
export function fechaYHora(instante: string | null | undefined): string {
  if (!instante) {
    return '—';
  }
  const fecha = new Date(instante);
  if (Number.isNaN(fecha.getTime())) {
    return instante;
  }
  return new Intl.DateTimeFormat('es-CR', {
    timeZone: ZONA,
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  })
    .format(fecha)
    .replace(',', '');
}

/** Un instante UTC → `09/10/2026`: el día de Costa Rica, no el del equipo. */
export function soloFecha(instante: string | null | undefined): string {
  return formatear(instante, { day: '2-digit', month: '2-digit', year: 'numeric' });
}

/** Un instante UTC → `08:30`: la hora de Costa Rica, en 24 horas. */
export function soloHora(instante: string | null | undefined): string {
  return formatear(instante, { hour: '2-digit', minute: '2-digit', hour12: false });
}

function formatear(
  instante: string | null | undefined,
  opciones: Intl.DateTimeFormatOptions,
): string {
  if (!instante) {
    return '—';
  }
  const fecha = new Date(instante);
  if (Number.isNaN(fecha.getTime())) {
    return instante;
  }
  return new Intl.DateTimeFormat('es-CR', { timeZone: ZONA, ...opciones }).format(fecha);
}
