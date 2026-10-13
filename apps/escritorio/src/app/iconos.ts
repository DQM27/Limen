import { inject } from '@angular/core';
import { MatIconRegistry } from '@angular/material/icon';
import { DomSanitizer } from '@angular/platform-browser';

/**
 * Íconos propios, para lo que Material no trae. "kof" es una tapa de botella
 * (corona de 12 dientes con un anillo grabado); se pinta con el color del texto.
 */
const ICONOS: Record<string, string> = {
  kof: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" fill-rule="evenodd" d="M12.00 1.00 L14.48 2.73 L17.50 2.47 L18.79 5.21 L21.53 6.50 L21.27 9.52 L23.00 12.00 L21.27 14.48 L21.53 17.50 L18.79 18.79 L17.50 21.53 L14.48 21.27 L12.00 23.00 L9.52 21.27 L6.50 21.53 L5.21 18.79 L2.47 17.50 L2.73 14.48 L1.00 12.00 L2.73 9.52 L2.47 6.50 L5.21 5.21 L6.50 2.47 L9.52 2.73 Z M18.60 12A6.6 6.6 0 1 0 5.40 12A6.6 6.6 0 1 0 18.60 12Z M17.40 12A5.4 5.4 0 1 0 6.60 12A5.4 5.4 0 1 0 17.40 12Z"/></svg>',
};

/** Registra los íconos propios para usarlos con svgIcon (por ejemplo, svgIcon="kof"). */
export function registrarIconos(): void {
  const registro = inject(MatIconRegistry);
  const sanitizador = inject(DomSanitizer);
  for (const [nombre, svg] of Object.entries(ICONOS)) {
    registro.addSvgIconLiteral(nombre, sanitizador.bypassSecurityTrustHtml(svg));
  }
}
