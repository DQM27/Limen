import { Injectable } from '@angular/core';
import {
  crearUsuario,
  editarUsuario,
  listarUsuarios,
  restablecerClaveDeUsuario,
  type Usuario,
  type UsuarioCreado,
} from '../panel';

/**
 * Lo que la pantalla de usuarios le pide al Worker. Es un servicio para que las pruebas lo
 * reemplacen por uno falso (ver `dialogo-usuario.spec.ts`); no decide nada.
 */
@Injectable({ providedIn: 'root' })
export class ApiUsuarios {
  listar(): Promise<Usuario[]> {
    return listarUsuarios();
  }

  crear(datos: { cedula: string; nombre: string }): Promise<UsuarioCreado> {
    return crearUsuario(datos);
  }

  editar(id: string, datos: { cedula: string; nombre: string; activo: boolean }): Promise<Usuario> {
    return editarUsuario(id, datos);
  }

  restablecerClave(id: string): Promise<string> {
    return restablecerClaveDeUsuario(id);
  }
}
