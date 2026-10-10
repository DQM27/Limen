import { Injectable, signal } from '@angular/core';
import { cambiarClave, cerrarSesion, iniciarSesion, usuarioActual } from './comandos';
import { enTauri } from './tauri';
import type { UsuarioActual } from './tipos';

/**
 * La sesión de este equipo, como señal para la interfaz. Quién puede hacer
 * qué lo decide el núcleo; esto sólo recuerda lo que respondió.
 */
@Injectable({ providedIn: 'root' })
export class SesionServicio {
  private readonly _usuario = signal<UsuarioActual | null>(null);

  /** Quién tiene la sesión; `null` sin sesión, mientras carga o fuera de la app. */
  readonly usuario = this._usuario.asReadonly();

  async cargar(): Promise<void> {
    if (!enTauri()) {
      return;
    }
    this._usuario.set(await usuarioActual());
  }

  async iniciar(cedula: string, clave: string): Promise<UsuarioActual> {
    const usuario = await iniciarSesion(cedula, clave);
    this._usuario.set(usuario);
    return usuario;
  }

  async cambiarClave(claveActual: string, clave: string): Promise<void> {
    await cambiarClave(claveActual, clave);
    this._usuario.update((usuario) =>
      usuario ? { ...usuario, debe_cambiar_clave: false } : usuario,
    );
  }

  async cerrar(): Promise<void> {
    await cerrarSesion();
    this._usuario.set(null);
  }
}
