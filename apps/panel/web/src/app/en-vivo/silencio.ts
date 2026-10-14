import { Injectable } from '@angular/core';

/** Cuánto se silencia el aviso en vivo de lo que esta misma página acaba de cambiar. */
const SILENCIO_MS = 8_000;

/**
 * Lo que esta página cambia ya se confirma en el momento (con su propio aviso): el aviso en vivo
 * del mismo cambio sería repetirlo. Antes de mandar un cambio se silencia su tabla unos segundos.
 */
@Injectable({ providedIn: 'root' })
export class Silencio {
  private readonly hasta = new Map<string, number>();

  silenciar(tabla: string, ms = SILENCIO_MS): void {
    this.hasta.set(tabla, Date.now() + ms);
  }

  estaSilenciada(tabla: string): boolean {
    const vence = this.hasta.get(tabla);
    if (vence === undefined) {
      return false;
    }
    if (Date.now() > vence) {
      this.hasta.delete(tabla);
      return false;
    }
    return true;
  }
}
