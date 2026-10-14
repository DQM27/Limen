import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { Subject } from 'rxjs';
import { describe, expect, it, vi } from 'vitest';
import { EnVivo } from '../en-vivo/en-vivo';
import type { AvisoDeTabla } from '../en-vivo/cliente-surreal';
import { Nucleo } from '../nucleo/nucleo';
import { ErrorPanel, type FiltroDeIngresos, type Ingreso } from '../panel';
import { ApiIngresos } from './api-ingresos';
import { IngresosPagina } from './ingresos-pagina';

class NucleoFalso extends Nucleo {
  filasQueCoinciden(): boolean[] {
    return [];
  }
  resaltar(): never[] {
    return [];
  }
  buscadorAlEscribir(texto: string): string {
    return texto;
  }
  nombreAlEscribir(texto: string): string {
    return texto;
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
    return texto;
  }
  fechaEsPraind(): boolean {
    return true;
  }
  praindVencido(): boolean {
    return false;
  }
}

const ADENTRO: Ingreso = {
  id: '1',
  via: 'CONTRATISTA',
  unidad: 'BRISAS',
  identidad: '111111111',
  nombre: 'ANA MORA',
  procedencia: 'ACME',
  placa: null,
  gafete: 7,
  sin_gafete: false,
  entrada: '2026-10-14T14:00:00Z',
  entrada_por: 'DANIEL QUINTANA',
  salida: null,
  salida_por: null,
};

// Los accesos rápidos los da el núcleo: la pantalla sólo los muestra.
const ATAJOS = {
  por_omision: 'ULTIMOS_6_MESES',
  atajos: [
    { codigo: 'HOY', etiqueta: 'Hoy', corta: 'Hoy', desde: '2026-10-14', hasta: '2026-10-14' },
    {
      codigo: 'ULTIMOS_6_MESES',
      etiqueta: 'Últimos 6 meses',
      corta: '6 meses',
      desde: '2026-04-14',
      hasta: null,
    },
  ],
};

async function abrir() {
  const cambios = new Subject<AvisoDeTabla>();
  const api = {
    atajos: vi.fn().mockResolvedValue(ATAJOS),
    unidades: vi.fn().mockResolvedValue([{ codigo: 'BRISAS', nombre: 'Brisas' }]),
    listar: vi.fn().mockResolvedValue({ ingresos: [ADENTRO], truncado: false }),
  };
  TestBed.configureTestingModule({
    providers: [
      { provide: Nucleo, useClass: NucleoFalso },
      { provide: ApiIngresos, useValue: api },
      {
        provide: EnVivo,
        useValue: { estado: signal('en_vivo'), cambios, reconectado: new Subject<void>() },
      },
    ],
  });
  const fixture = TestBed.createComponent(IngresosPagina);
  await fixture.whenStable();
  fixture.detectChanges();
  const raiz = fixture.nativeElement as HTMLElement;
  const boton = (texto: string): HTMLButtonElement => {
    const todos = [...raiz.querySelectorAll<HTMLButtonElement>('mat-button-toggle button')];
    const encontrado = todos.find((b) => b.textContent?.includes(texto));
    if (!encontrado) {
      throw new Error(`no hay un botón «${texto}»`);
    }
    return encontrado;
  };
  const filtroPedido = (): FiltroDeIngresos => api.listar.mock.lastCall?.[0] as FiltroDeIngresos;
  return { fixture, raiz, api, cambios, boton, filtroPedido };
}

describe('IngresosPagina', () => {
  it('abre en «Adentro ahora», sin fechas: lo que está pasando', async () => {
    const { api, filtroPedido, raiz } = await abrir();

    expect(api.listar).toHaveBeenCalledTimes(1);
    expect(filtroPedido()).toMatchObject({ estado: 'adentro', via: '', unidad: '' });
    expect(raiz.textContent).not.toContain('Rango de fechas');
  });

  it('«Historial» pide el rango por omisión que dio el núcleo y muestra los accesos rápidos', async () => {
    const { fixture, boton, filtroPedido, raiz } = await abrir();

    boton('Historial').click();
    await fixture.whenStable();
    fixture.detectChanges();

    expect(filtroPedido()).toMatchObject({
      estado: 'historial',
      desde: '2026-04-14',
      hasta: '',
    });
    expect(raiz.textContent).toContain('6 meses');
    expect(raiz.textContent).toContain('Hoy');
  });

  it('elegir un acceso rápido o una vía vuelve a pedir con ese filtro', async () => {
    const { fixture, boton, filtroPedido } = await abrir();
    boton('Historial').click();
    await fixture.whenStable();
    fixture.detectChanges();

    boton('Hoy').click();
    await fixture.whenStable();
    expect(filtroPedido()).toMatchObject({ desde: '2026-10-14', hasta: '2026-10-14' });

    boton('Proveedores').click();
    await fixture.whenStable();
    expect(filtroPedido()).toMatchObject({ via: 'PROVEEDOR' });
  });

  it('cada entrada o salida (un hecho) vuelve a leer al instante', async () => {
    const { api, cambios } = await abrir();
    const antes = api.listar.mock.calls.length;
    const esperar = (ms: number): Promise<void> => new Promise((fin) => setTimeout(fin, ms));

    cambios.next({ tabla: 'usuario' } as AvisoDeTabla);
    await esperar(450);
    expect(api.listar.mock.calls.length, 'un usuario no es un ingreso').toBe(antes);

    cambios.next({ tabla: 'hecho' } as AvisoDeTabla);
    await esperar(450);
    expect(api.listar.mock.calls.length).toBe(antes + 1);
  });

  it('el aviso de que hay más movimientos y el error de rango salen de lo que responde el Worker', async () => {
    const { fixture, api, boton, raiz } = await abrir();
    api.listar.mockResolvedValueOnce({ ingresos: [ADENTRO], truncado: true });
    boton('Historial').click();
    await fixture.whenStable();
    fixture.detectChanges();
    expect(raiz.textContent).toContain('Hay más movimientos de los que se muestran');

    api.listar.mockRejectedValueOnce(
      new ErrorPanel('problemas', [
        { codigo: 'rango_invertido', campo: 'hasta', mensaje: 'MENSAJE DEL NÚCLEO: rango' },
      ]),
    );
    boton('Hoy').click();
    await fixture.whenStable();
    fixture.detectChanges();
    expect(raiz.textContent).toContain('MENSAJE DEL NÚCLEO: rango');
  });
});
