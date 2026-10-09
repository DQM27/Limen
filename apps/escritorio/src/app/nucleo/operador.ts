import { Injectable, signal } from '@angular/core';
import { enTauri, invocar } from './tauri';

/** TEMPORAL hasta el inicio de sesión real (bloque L): el operador de este equipo. */
export interface Operador {
  id: string;
  nombre: string;
}

@Injectable({ providedIn: 'root' })
export class OperadorServicio {
  private readonly _operador = signal<Operador | null>(null);

  /** El operador de este equipo; `null` mientras carga o fuera de la app. */
  readonly operador = this._operador.asReadonly();

  async cargar(): Promise<void> {
    if (!enTauri()) {
      return;
    }
    this._operador.set(await invocar<Operador | null>('operador_actual'));
  }
}
