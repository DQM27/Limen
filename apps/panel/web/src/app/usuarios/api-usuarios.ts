import { Injectable, inject } from '@angular/core';
import {
  crearUsuario,
  editarUsuario,
  listarUsuarios,
  restablecerClaveDeUsuario,
  type Usuario,
  type UsuarioCreado,
} from '../panel';
import { Silencio } from '../en-vivo/silencio';

/**
 * Lo que la pantalla de usuarios le pide al Worker. Es un servicio para que las pruebas lo
 * reemplacen por uno falso (ver `dialogo-usuario.spec.ts`); no decide nada.
 */
@Injectable({ providedIn: 'root' })
export class ApiUsuarios {
  private readonly silencio = inject(Silencio);

  listar(): Promise<Usuario[]> {
    return listarUsuarios();
  }

  crear(datos: { cedula: string; nombre: string }): Promise<UsuarioCreado> {
    this.silencio.silenciar('usuario');
    return crearUsuario(datos);
  }

  editar(id: string, datos: { cedula: string; nombre: string; activo: boolean }): Promise<Usuario> {
    this.silencio.silenciar('usuario');
    return editarUsuario(id, datos);
  }

  restablecerClave(id: string): Promise<string> {
    this.silencio.silenciar('usuario');
    return restablecerClaveDeUsuario(id);
  }
}
