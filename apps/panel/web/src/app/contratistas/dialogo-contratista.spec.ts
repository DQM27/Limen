import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { describe, expect, it, vi } from 'vitest';
import { Nucleo, type CampoDeTexto } from '../nucleo/nucleo';
import { ErrorPanel, type Contratista } from '../panel';
import { ApiContratistas } from './api-contratistas';
import { DialogoContratista, type DatosDialogoContratista } from './dialogo-contratista';

/**
 * Un núcleo de mentira: lo que «decide» es predecible, para comprobar que la pantalla sólo lo
 * muestra y no inventa nada propio.
 */
class NucleoFalso extends Nucleo {
  filasQueCoinciden(buscado: string, filas: CampoDeTexto[][]): boolean[] {
    return filas.map((f) => f.some((c) => c.texto.includes(buscado.toUpperCase())));
  }
  resaltar(): never[] {
    return [];
  }
  nombreAlEscribir(texto: string): string {
    return texto.toUpperCase();
  }
  datosDeUsuario(): never {
    throw new Error('no se usa');
  }
  estadoDeConexion(): never {
    throw new Error('no se usa');
  }
  mensajeDeError(): string {
    return '';
  }
  empresaAlEscribir(texto: string): string {
    return texto.toUpperCase();
  }
  praindVencido(): boolean {
    return false;
  }
}

const ACME = { id: '0199c7a2-0000-7000-8000-0000000000aa', nombre: 'ACME' };
const OTRA = { id: '0199c7a2-0000-7000-8000-0000000000bb', nombre: 'OTRA S.A.' };
const JOSE: Contratista = {
  id: '0199c7a2-0000-7000-8000-000000000001',
  cedula: '111111111',
  nombre: 'JOSE PEÑA',
  empresa_id: ACME.id,
  empresa_nombre: ACME.nombre,
  tipo_ingreso: 'PRAIND',
  fecha_vencimiento_praind: '2999-01-01',
  tiene_acceso: true,
};

function abrir(datos: Partial<DatosDialogoContratista> = {}) {
  const cerrar = vi.fn();
  const api = { registrar: vi.fn(), editar: vi.fn(), crearEmpresa: vi.fn() };
  TestBed.configureTestingModule({
    providers: [
      { provide: Nucleo, useClass: NucleoFalso },
      { provide: ApiContratistas, useValue: api },
      { provide: MatDialogRef, useValue: { close: cerrar } },
      { provide: MAT_DIALOG_DATA, useValue: { empresas: [ACME, OTRA], ...datos } },
    ],
  });
  const fixture = TestBed.createComponent(DialogoContratista);
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

describe('DialogoContratista', () => {
  it('el nombre se ve como lo dice el núcleo mientras se escribe', () => {
    const { escribir, campo } = abrir();
    escribir('nombre', 'ana mora');
    expect(campo('nombre').value).toBe('ANA MORA');
  });

  it('al editar precarga al contratista y manda lo escrito tal cual, sin decidir nada', async () => {
    const { api, escribir, enviar, cerrar } = abrir({ contratista: JOSE });
    api.editar.mockResolvedValue({ ...JOSE, nombre: 'OTRO' });
    escribir('cedula', '1-1111-1111');
    await enviar();

    expect(api.editar).toHaveBeenCalledWith(JOSE.id, {
      cedula: '1-1111-1111',
      nombre: 'JOSE PEÑA',
      empresa_id: ACME.id,
      tipo_ingreso: 'PRAIND',
      fecha_vencimiento_praind: '2999-01-01',
      tiene_acceso: true,
    });
    expect(cerrar).toHaveBeenCalledWith({
      tipo: 'editado',
      contratista: { ...JOSE, nombre: 'OTRO' },
    });
  });

  it('marca cada campo con el mensaje que da el núcleo, todos a la vez', async () => {
    const { api, enviar, texto, cerrar } = abrir();
    api.registrar.mockRejectedValue(
      new ErrorPanel('problemas', [
        { codigo: 'cedula_vacia', campo: 'cedula', mensaje: 'MENSAJE DEL NÚCLEO: cédula' },
        {
          codigo: 'empresa_requerida',
          campo: 'empresa_id',
          mensaje: 'MENSAJE DEL NÚCLEO: empresa',
        },
      ]),
    );
    await enviar();

    expect(texto()).toContain('MENSAJE DEL NÚCLEO: cédula');
    expect(texto()).toContain('MENSAJE DEL NÚCLEO: empresa');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('un fallo de la nube se avisa y no cierra el diálogo', async () => {
    const { api, enviar, texto, cerrar } = abrir();
    api.registrar.mockRejectedValue(new ErrorPanel('nube'));
    await enviar();
    expect(texto()).toContain('El panel no pudo hablar con la base de datos');
    expect(cerrar).not.toHaveBeenCalled();
  });
});
