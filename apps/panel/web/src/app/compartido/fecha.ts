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
