const FORMATO = new Intl.DateTimeFormat('es-CR', {
  day: '2-digit',
  month: '2-digit',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  // Hora militar: 00 a 23, sin a. m. ni p. m.
  hourCycle: 'h23',
});

/** Una fecha con hora como se ve en todo el panel: `14/10/2026 17:26:20` (24 horas). */
export function fechaHora(valor: string | null | undefined): string {
  if (!valor) {
    return '';
  }
  const fecha = new Date(valor);
  if (Number.isNaN(fecha.getTime())) {
    return '';
  }
  return FORMATO.format(fecha).replace(',', '');
}

const SOLO_HORA = new Intl.DateTimeFormat('es-CR', {
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hourCycle: 'h23',
});

/** Sólo la hora, en 24 horas: `17:26:20`. */
export function hora24(valor: string | null | undefined): string {
  if (!valor) {
    return '';
  }
  const fecha = new Date(valor);
  return Number.isNaN(fecha.getTime()) ? '' : SOLO_HORA.format(fecha);
}

/** Un día (`AAAA-MM-DD`) como se ve en todo el panel: `31/12/2026`. */
export function soloFecha(valor: string | null | undefined): string {
  const [año, mes, dia] = (valor ?? '').split('-');
  return año && mes && dia ? `${dia}/${mes}/${año}` : '';
}
