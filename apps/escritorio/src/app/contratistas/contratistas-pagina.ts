import { ChangeDetectionStrategy, Component } from '@angular/core';

@Component({
  selector: 'app-contratistas-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <h1>Contratistas</h1>
    <p>La grilla y el formulario de registro llegan en el siguiente paso.</p>
  `,
})
export class ContratistasPagina {}
