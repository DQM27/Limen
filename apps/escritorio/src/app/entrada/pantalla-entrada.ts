import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  computed,
  ElementRef,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatCheckboxModule } from '@angular/material/checkbox';
import { SesionServicio } from '../nucleo/sesion';
import { esErrorApp } from '../nucleo/tauri';
import { salir } from '../nucleo/ventana';

const CLAVE_USUARIO_RECORDADO = 'limen.usuario-recordado';

type Campo = 'cedula' | 'clave' | 'clave_actual' | 'confirmacion';

/** Lo único que se recuerda en el equipo es la cédula, nunca la clave. */
function leerUsuarioRecordado(): string {
  try {
    return localStorage.getItem(CLAVE_USUARIO_RECORDADO) ?? '';
  } catch {
    return '';
  }
}

function guardarUsuarioRecordado(cedula: string | null): void {
  try {
    if (cedula === null) {
      localStorage.removeItem(CLAVE_USUARIO_RECORDADO);
    } else {
      localStorage.setItem(CLAVE_USUARIO_RECORDADO, cedula);
    }
  } catch {
    // Sin almacenamiento, simplemente no se recuerda.
  }
}

/**
 * La ventana de entrada: inicio de sesión (cédula y clave) y, si el núcleo
 * pide una clave nueva (`debe_cambiar_clave`), el cambio de clave. No decide
 * nada: muestra lo que responde el núcleo, con su mensaje y su campo.
 */
@Component({
  selector: 'app-pantalla-entrada',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatCheckboxModule],
  templateUrl: './pantalla-entrada.html',
  styleUrl: './pantalla-entrada.scss',
})
export class PantallaEntrada {
  private readonly sesion = inject(SesionServicio);

  /** Con una clave temporal sólo se puede cambiarla. */
  protected readonly cambiando = computed(() => this.sesion.usuario()?.debe_cambiar_clave === true);

  protected readonly cedula = signal(leerUsuarioRecordado());
  protected readonly recordar = signal(this.cedula() !== '');
  protected readonly clave = signal('');
  protected readonly claveActual = signal('');
  protected readonly confirmacion = signal('');
  protected readonly verClave = signal(false);

  protected readonly enviando = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);
  protected readonly errorCampo = signal<Partial<Record<Campo, string>>>({});

  private readonly campoCedula = viewChild<ElementRef<HTMLInputElement>>('campoCedula');
  private readonly campoClave = viewChild<ElementRef<HTMLInputElement>>('campoClave');

  constructor() {
    // Con la cédula recordada, el foco va directo a la clave.
    afterNextRender(() => {
      (this.cedula() === '' ? this.campoCedula() : this.campoClave())?.nativeElement.focus();
    });
  }

  protected escribir(campo: 'cedula' | 'clave' | 'claveActual' | 'confirmacion', e: Event): void {
    this[campo].set((e.target as HTMLInputElement).value);
  }

  protected async enviar(evento: Event): Promise<void> {
    evento.preventDefault();
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.errorCampo.set({});
    if (this.cambiando()) {
      await this.cambiarClave();
    } else {
      await this.entrar();
    }
  }

  protected async cancelar(): Promise<void> {
    if (this.cambiando()) {
      await this.sesion.cerrar();
      this.limpiarClaves();
    } else {
      await salir();
    }
  }

  private async entrar(): Promise<void> {
    this.enviando.set(true);
    try {
      await this.sesion.iniciar(this.cedula().trim(), this.clave());
      guardarUsuarioRecordado(this.recordar() ? this.cedula().trim() : null);
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.clave.set('');
      // El campo se vacía a mano: lo escrito no pasó por un ciclo de pintado,
      // y sin esto la clave rechazada se quedaría a la vista.
      const campo = this.campoClave();
      if (campo) {
        campo.nativeElement.value = '';
      }
      this.enviando.set(false);
    }
  }

  private async cambiarClave(): Promise<void> {
    if (this.clave() !== this.confirmacion()) {
      this.errorCampo.set({ confirmacion: 'Las dos claves nuevas no coinciden.' });
      return;
    }
    this.enviando.set(true);
    try {
      await this.sesion.cambiarClave(this.claveActual(), this.clave());
      this.limpiarClaves();
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  private limpiarClaves(): void {
    this.claveActual.set('');
    this.clave.set('');
    this.confirmacion.set('');
  }

  /** El mensaje del núcleo, junto a su campo si lo trae; si no, el general. */
  private mostrar(error: unknown): void {
    if (!esErrorApp(error)) {
      this.errorGeneral.set('No se pudo completar la operación.');
      return;
    }
    const campo = error.campo;
    if (campo === 'cedula' || campo === 'clave' || campo === 'clave_actual') {
      this.errorCampo.set({ [campo]: error.mensaje });
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
