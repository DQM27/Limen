import { afterNextRender, type ElementRef, type Injector } from '@angular/core';

/**
 * Lleva el foco al primer campo con error, cuando el formulario ya pintó los
 * mensajes: quien captura no tiene que buscar qué falló.
 */
export function enfocarPrimerError(anfitrion: ElementRef<HTMLElement>, injector: Injector): void {
  afterNextRender(
    () =>
      anfitrion.nativeElement
        .querySelector<HTMLElement>(
          '.mat-form-field-invalid input, .mat-form-field-invalid mat-select',
        )
        ?.focus(),
    { injector },
  );
}
