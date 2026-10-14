import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Nucleo, type ResultadoDeDatos } from '../nucleo/nucleo';
import { ErrorPanel, type Usuario, type UsuarioCreado } from '../panel';
import { ApiUsuarios } from './api-usuarios';
import { DialogoUsuario, type DatosDialogoUsuario } from './dialogo-usuario';

/**
 * Un núcleo de mentira: lo que «decide» es predecible, para comprobar que la pantalla sólo
 * lo muestra y no inventa nada propio.
 */
class NucleoFalso extends Nucleo {
  filasQueCoinciden(): boolean[] {
    return [];
  }
  resaltar(): never[] {
    return [];
  }
  buscadorAlEscribir(texto: string): string {
    return texto.toUpperCase();
  }
  nombreAlEscribir(texto: string): string {
    return texto.toUpperCase();
  }
  datosDeUsuario(cedula: string, nombre: string): ResultadoDeDatos {
    if (cedula.trim() === '') {
      return { ok: false, codigo: 'cedula_vacia', mensaje: 'MENSAJE DEL NÚCLEO: cédula vacía' };
    }
    return {
      ok: true,
      datos: { cedula: cedula.replaceAll('-', ''), nombre: nombre.trim().toUpperCase() },
    };
  }
  estadoDeConexion(): never {
    throw new Error('no se usa en estas pruebas');
  }
  empresaAlEscribir(texto: string): string {
    return texto;
  }
  praindVencido(): boolean {
    return false;
  }
  mensajeDeError(codigo: string): string {
    return codigo === 'usuario_cedula_repetida' ? 'MENSAJE DEL NÚCLEO: ya existe' : '';
  }
}

const ANA: Usuario = {
  id: '0199c7a2-0000-7000-8000-000000000001',
  cedula: '111111111',
  nombre: 'ANA MORA',
  activo: true,
  debe_cambiar_clave: false,
};

function abrir(datos: DatosDialogoUsuario = {}) {
  const cerrar = vi.fn();
  const api = { crear: vi.fn(), editar: vi.fn() };
  TestBed.configureTestingModule({
    providers: [
      { provide: Nucleo, useClass: NucleoFalso },
      { provide: ApiUsuarios, useValue: api },
      { provide: MatDialogRef, useValue: { close: cerrar } },
      { provide: MAT_DIALOG_DATA, useValue: datos },
    ],
  });
  const fixture = TestBed.createComponent(DialogoUsuario);
  fixture.detectChanges();
  const campo = (nombre: string): HTMLInputElement =>
    fixture.nativeElement.querySelector(`input[name="${nombre}"]`) as HTMLInputElement;
  const escribir = (nombre: string, valor: string): void => {
    campo(nombre).value = valor;
    campo(nombre).dispatchEvent(new Event('input'));
    fixture.detectChanges();
  };
  const texto = (): string => fixture.nativeElement.textContent as string;
  const enviar = async (): Promise<void> => {
    (fixture.nativeElement.querySelector('form') as HTMLFormElement).dispatchEvent(
      new Event('submit', { cancelable: true }),
    );
    await fixture.whenStable();
    fixture.detectChanges();
  };
  return { api, campo, escribir, texto, enviar, cerrar };
}

describe('DialogoUsuario', () => {
  beforeEach(() => TestBed.resetTestingModule());

  it('el nombre se ve en mayúscula mientras se escribe: lo dice el núcleo', () => {
    const { escribir, campo } = abrir();
    escribir('nombre', 'ana mora');
    expect(campo('nombre').value).toBe('ANA MORA');
  });

  it('muestra cómo quedarán la cédula y el nombre según el núcleo', () => {
    const { escribir, texto } = abrir();
    escribir('cedula', '1-1111-1111');
    escribir('nombre', 'ana');
    expect(texto()).toContain('111111111');
    expect(texto()).toContain('ANA');
  });

  it('muestra el mensaje del núcleo cuando lo escrito no vale, y no manda nada', async () => {
    const { api, enviar, texto } = abrir();
    await enviar();
    expect(texto()).toContain('MENSAJE DEL NÚCLEO: cédula vacía');
    expect(api.crear).not.toHaveBeenCalled();
  });

  it('crea con lo que escribió la persona y devuelve la clave temporal', async () => {
    const creado: UsuarioCreado = { usuario: ANA, clave_temporal: '23456789AB' };
    const { api, escribir, enviar, cerrar } = abrir();
    api.crear.mockResolvedValue(creado);
    escribir('cedula', '1-1111-1111');
    escribir('nombre', 'ana mora');
    await enviar();
    expect(api.crear).toHaveBeenCalledWith({ cedula: '1-1111-1111', nombre: 'ANA MORA' });
    expect(cerrar).toHaveBeenCalledWith({ tipo: 'creado', creado });
  });

  it('una cédula repetida la decide la nube y el mensaje es el del núcleo', async () => {
    const { api, escribir, enviar, texto, cerrar } = abrir();
    api.crear.mockRejectedValue(new ErrorPanel('usuario_cedula_repetida'));
    escribir('cedula', '1-1111-1111');
    escribir('nombre', 'ana');
    await enviar();
    expect(texto()).toContain('MENSAJE DEL NÚCLEO: ya existe');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('editar manda la cédula y el nombre nuevos y conserva si está activo', async () => {
    const editado: Usuario = { ...ANA, cedula: '222222222' };
    const { api, escribir, enviar, cerrar } = abrir({ usuario: ANA });
    api.editar.mockResolvedValue(editado);
    escribir('cedula', '2-2222-2222');
    await enviar();
    expect(api.editar).toHaveBeenCalledWith(ANA.id, {
      cedula: '2-2222-2222',
      nombre: 'ANA MORA',
      activo: true,
    });
    expect(cerrar).toHaveBeenCalledWith({ tipo: 'editado', usuario: editado });
  });
});
