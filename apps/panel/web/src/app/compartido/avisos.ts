import { ChangeDetectionStrategy, Component, inject, Injectable, signal } from '@angular/core';
import { MatIconModule } from '@angular/material/icon';

/** Qué tipo de aviso es: decide su color, su ícono, su título y cuánto dura. */
export type TipoAviso = 'exito' | 'error' | 'info';

export interface Aviso {
  id: number;
  tipo: TipoAviso;
  titulo: string;
  mensaje: string;
  /** Ya se está yendo: se anima la salida antes de quitarlo. */
  saliendo: boolean;
}

const ICONOS: Record<TipoAviso, string> = {
  exito: 'check_circle',
  error: 'error',
  info: 'info',
};

const TITULOS: Record<TipoAviso, string> = {
  exito: 'Listo',
  error: 'No se pudo',
  info: 'Aviso',
};

/** Cuánto se queda cada tipo en pantalla: un error se lee con más calma. */
const DURACION_MS: Record<TipoAviso, number> = {
  exito: 4500,
  info: 4500,
  error: 8000,
};

/** Cuánto dura la animación de salida (debe coincidir con la del estilo). */
const SALIDA_MS = 260;

/** Cuántos avisos se ven a la vez: al llegar uno más, se va el más viejo. */
const MAXIMO_VISIBLES = 4;

/**
 * Los avisos de la app, como las notificaciones de macOS: arriba a la
 * derecha, entran deslizándose desde el borde, se apilan (el más nuevo
 * arriba) y se van solos. Para mostrarlos, ver {@link AvisosPanel}.
 */
@Injectable({ providedIn: 'root' })
export class Avisos {
  private readonly _avisos = signal<Aviso[]>([]);
  private siguiente = 0;

  /** Los que están a la vista, del más nuevo al más viejo. */
  readonly avisos = this._avisos.asReadonly();

  /** Algo salió bien. */
  exito(mensaje: string, titulo?: string): void {
    this.mostrar('exito', mensaje, titulo);
  }

  /** Algo falló: dura más, para que se alcance a leer. */
  error(mensaje: string, titulo?: string): void {
    this.mostrar('error', mensaje, titulo);
  }

  /** Algo que informar, ni bueno ni malo (por ejemplo, "Sin cambios"). */
  info(mensaje: string, titulo?: string): void {
    this.mostrar('info', mensaje, titulo);
  }

  /** Lo quita con su animación de salida. */
  cerrar(id: number): void {
    const existe = this._avisos().some((a) => a.id === id && !a.saliendo);
    if (!existe) {
      return;
    }
    this._avisos.update((lista) => lista.map((a) => (a.id === id ? { ...a, saliendo: true } : a)));
    setTimeout(() => this._avisos.update((lista) => lista.filter((a) => a.id !== id)), SALIDA_MS);
  }

  private mostrar(tipo: TipoAviso, mensaje: string, titulo?: string): void {
    const id = ++this.siguiente;
    this._avisos.update((lista) =>
      [{ id, tipo, titulo: titulo ?? TITULOS[tipo], mensaje, saliendo: false }, ...lista].slice(
        0,
        MAXIMO_VISIBLES,
      ),
    );
    setTimeout(() => this.cerrar(id), DURACION_MS[tipo]);
  }
}

/**
 * El panel donde se ven los avisos: va una sola vez, en la raíz de la app.
 * Un clic en un aviso lo cierra. Los errores se anuncian a los lectores de
 * pantalla de inmediato; el resto, con calma.
 */
@Component({
  selector: 'app-avisos',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatIconModule],
  templateUrl: './avisos.html',
  styleUrl: './avisos.scss',
})
export class AvisosPanel {
  protected readonly servicio = inject(Avisos);
  protected readonly iconos = ICONOS;
}
