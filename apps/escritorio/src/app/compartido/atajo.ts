import { Directive, ElementRef, inject, input } from '@angular/core';

/** Un atajo como `F2`, `Ctrl+N` o `/`, ya separado en sus partes. */
interface Combinacion {
  tecla: string;
  ctrl: boolean;
  alt: boolean;
  mayus: boolean;
}

function leer(atajo: string): Combinacion {
  const partes = atajo.split('+').map((parte) => parte.trim().toLowerCase());
  const tecla = partes.pop() ?? '';
  return {
    tecla,
    ctrl: partes.includes('ctrl'),
    alt: partes.includes('alt'),
    mayus: partes.includes('shift'),
  };
}

/**
 * ¿La tecla pulsada es este atajo? Una tecla suelta ("/") se acepta con o sin
 * Mayús, porque en algunos teclados se escribe con ella; las demás
 * combinaciones deben coincidir exactamente. Ctrl vale igual que Cmd.
 */
export function coincide(evento: KeyboardEvent, atajo: string): boolean {
  const esperado = leer(atajo);
  if (evento.key.toLowerCase() !== esperado.tecla) {
    return false;
  }
  const ctrl = evento.ctrlKey || evento.metaKey;
  const suelta = esperado.tecla.length === 1 && !esperado.ctrl && !esperado.alt;
  return (
    ctrl === esperado.ctrl &&
    evento.altKey === esperado.alt &&
    (suelta || evento.shiftKey === esperado.mayus)
  );
}

/** ¿Se está escribiendo en un campo? Entonces una tecla suelta es texto, no un atajo. */
function escribiendoEn(destino: EventTarget | null): boolean {
  const elemento = destino as HTMLElement | null;
  return (
    !!elemento &&
    (['INPUT', 'TEXTAREA', 'SELECT'].includes(elemento.tagName) || elemento.isContentEditable)
  );
}

/** Mientras hay un modal abierto, los atajos de la pantalla de atrás no valen. */
function hayModalAbierto(): boolean {
  return document.querySelector('.mat-mdc-dialog-container') !== null;
}

/**
 * Le da un atajo de teclado a un botón (lo pulsa) o a un campo (lo enfoca).
 * Sólo vale mientras la pantalla está a la vista y no hay un modal encima.
 * `<button appAtajo="F2">` · `<input appAtajo="/">`
 */
@Directive({
  selector: '[appAtajo]',
  host: {
    '(document:keydown)': 'alTeclar($event)',
    '[attr.aria-keyshortcuts]': 'atajo()',
  },
})
export class Atajo {
  readonly atajo = input.required<string>({ alias: 'appAtajo' });
  private readonly elemento = inject<ElementRef<HTMLElement>>(ElementRef);

  protected alTeclar(evento: KeyboardEvent): void {
    const atajo = this.atajo();
    if (evento.defaultPrevented || !coincide(evento, atajo) || hayModalAbierto()) {
      return;
    }
    const sinModificador = !evento.ctrlKey && !evento.metaKey && !evento.altKey;
    if (atajo.length === 1 && sinModificador && escribiendoEn(evento.target)) {
      return;
    }
    const elemento = this.elemento.nativeElement;
    if (elemento.hasAttribute('disabled') || elemento.getAttribute('aria-disabled') === 'true') {
      return;
    }
    evento.preventDefault();
    if (elemento instanceof HTMLInputElement) {
      elemento.focus();
      elemento.select();
    } else {
      elemento.click();
    }
  }
}
