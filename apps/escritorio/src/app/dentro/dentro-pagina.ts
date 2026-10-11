import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ListaDentro } from './lista-dentro';

/**
 * Quién está adentro ahora, por las cuatro vías juntas. Desde aquí se
 * registran las salidas; el ingreso se registra en la sección de cada vía.
 */
@Component({
  selector: 'app-dentro-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ListaDentro],
  template: `<app-lista-dentro />`,
  styles: `
    :host {
      display: flex;
      flex: 1;
      flex-direction: column;
      min-height: 0;
    }
  `,
})
export class DentroPagina {}
