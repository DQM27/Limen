import { Injectable, signal } from '@angular/core';
import {
  cambiarContrasena,
  cerrarSesion,
  crearPrimerUsuario,
  iniciarSesion,
  usuarioActual,
} from './comandos';
import { enTauri } from './tauri';
import type { UsuarioActual, UsuarioEntrada } from './tipos';

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

  async iniciar(cedula: string, contrasena: string): Promise<UsuarioActual> {
    const usuario = await iniciarSesion(cedula, contrasena);
    this._usuario.set(usuario);
    return usuario;
  }

  async crearPrimerUsuario(entrada: UsuarioEntrada): Promise<UsuarioActual> {
    const usuario = await crearPrimerUsuario(entrada);
    this._usuario.set(usuario);
    return usuario;
  }

  async cambiarContrasena(contrasenaActual: string, contrasena: string): Promise<void> {
    await cambiarContrasena(contrasenaActual, contrasena);
    this._usuario.update((usuario) =>
      usuario ? { ...usuario, debe_cambiar_contrasena: false } : usuario,
    );
  }

  async cerrar(): Promise<void> {
    await cerrarSesion();
    this._usuario.set(null);
  }
}
