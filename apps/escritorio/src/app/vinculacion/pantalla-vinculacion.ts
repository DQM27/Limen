import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { reiniciar, vincularEquipo } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import { salir } from '../nucleo/ventana';

/**
 * Primer arranque: el equipo se vincula con el código que el administrador emite en el
 * panel (reglas M6 a M8). No decide nada: manda lo escrito y muestra lo que responde el
 * núcleo, junto al campo si trae. Tras vincular, la app se reinicia para abrir la base
 * con la identidad nueva.
 */
@Component({
  selector: 'app-pantalla-vinculacion',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule],
  templateUrl: './pantalla-vinculacion.html',
  styleUrl: '../entrada/pantalla-entrada.scss',
})
export class PantallaVinculacion {
  /** En desarrollo se puede seguir sin vincular, con los usuarios de la semilla. */
  readonly desarrollo = input(false);
  readonly omitir = output<void>();

  protected readonly codigo = signal('');
  protected readonly enviando = signal(false);
  protected readonly vinculado = signal(false);
  /** En desarrollo la app no se reinicia sola: se le pide a la persona que la abra de nuevo. */
  protected readonly reinicioManual = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);
  protected readonly errorCodigo = signal<string | null>(null);

  private readonly campoCodigo = viewChild<ElementRef<HTMLInputElement>>('campoCodigo');

  constructor() {
    afterNextRender(() => this.campoCodigo()?.nativeElement.focus());
  }

  protected escribir(evento: Event): void {
    this.codigo.set((evento.target as HTMLInputElement).value);
  }

  protected async enviar(evento: Event): Promise<void> {
    evento.preventDefault();
    if (this.enviando() || this.vinculado()) {
      return;
    }
    this.errorGeneral.set(null);
    this.errorCodigo.set(null);
    this.enviando.set(true);
    try {
      await vincularEquipo(this.codigo());
      this.vinculado.set(true);
      this.reinicioManual.set(!(await reiniciar()));
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): Promise<void> {
    return salir();
  }

  /** El mensaje del núcleo, junto a su campo si lo trae; si no, el general. */
  private mostrar(error: unknown): void {
    if (!esErrorApp(error)) {
      this.errorGeneral.set('No se pudo completar la operación.');
    } else if (error.campo === 'codigo') {
      this.errorCodigo.set(error.mensaje);
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
