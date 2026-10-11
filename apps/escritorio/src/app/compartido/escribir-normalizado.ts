import type { FormControl } from '@angular/forms';

/**
 * Mientras se escribe, el texto se ve como quedará al guardarse (mayúsculas,
 * sin tildes…): la forma la da el dominio, no la interfaz. El cursor se queda
 * donde estaba, y sólo vale la última respuesta (las que llegan tarde, porque
 * se siguió escribiendo, se descartan). Cada campo usa su propia instancia.
 */
export class NormalizadorAlEscribir {
  private turno = 0;

  async alEscribir(
    evento: Event,
    control: FormControl<string>,
    comoQueda: (texto: string) => Promise<string>,
  ): Promise<void> {
    const campo = evento.target as HTMLInputElement;
    const texto = campo.value;
    const cursor = campo.selectionStart;
    const mio = ++this.turno;
    const queda = await comoQueda(texto).catch(() => texto);
    if (mio !== this.turno || queda === texto || campo.value !== texto) {
      return;
    }
    control.setValue(queda);
    campo.setSelectionRange(cursor, cursor);
  }
}
