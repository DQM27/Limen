import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import type { Gafete } from '../nucleo/tipos';
import { DialogoGestionGafete } from './dialogo-gestion-gafete';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

function gafete(cambios: Partial<Gafete> = {}): Gafete {
  return {
    tipo: 'CONTRATISTA',
    numero: 25,
    estado: 'DISPONIBLE',
    prestado: false,
    portador_contratista_id: null,
    portador_cedula: null,
    portador_personal_id: null,
    asignado_nombre: null,
    asignado_identidad: null,
    ...cambios,
  };
}

async function crear(datos: Gafete, teclas = new Subject<KeyboardEvent>()) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
      { provide: MAT_DIALOG_DATA, useValue: { gafete: datos } },
    ],
  });
  const fixture = TestBed.createComponent(DialogoGestionGafete);
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const boton = (texto: string) =>
    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.trim() === texto);
  const pulsar = async (texto: string) => {
    const b = boton(texto);
    if (!b) {
      throw new Error(`no hay botón «${texto}»`);
    }
    b.click();
    await fixture.whenStable();
    fixture.detectChanges();
  };
  const enviarFormulario = async () => {
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  return {
    fixture,
    raiz,
    cerrar,
    boton,
    pulsar,
    enviarFormulario,
    portador: fixture.componentInstance.portador,
  };
}

describe('modal de un gafete', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue([{ campo: 'estado', antes: 'a', despues: 'b' }]);
  });

  it('muestra su número, tipo y estado', async () => {
    const { raiz } = await crear(gafete({ tipo: 'PROVEEDOR', numero: 7 }));

    expect(raiz.querySelector('h2')?.textContent).toContain('Gafete 7');
    expect(raiz.querySelector('.datos')?.textContent).toContain('Proveedor');
    expect(raiz.querySelector('.datos')?.textContent).toContain('Disponible');
  });

  it('uno prestado sólo informa quién lo tiene y que se libera desde Dentro', async () => {
    const { raiz, boton } = await crear(
      gafete({
        prestado: true,
        asignado_nombre: 'ANA SOLANO',
        asignado_identidad: '111111111',
      }),
    );

    expect(raiz.querySelector('.datos')?.textContent).toContain('Lo tiene');
    expect(raiz.querySelector('.datos')?.textContent).toContain('ANA SOLANO (111111111)');
    expect(raiz.textContent).toContain('«Dentro»');
    expect(boton('Marcar perdido')).toBeUndefined();
    expect(boton('Dar de baja')).toBeUndefined();
  });

  it('uno disponible ofrece marcarlo perdido o darlo de baja', async () => {
    const { boton } = await crear(gafete());

    expect(boton('Marcar perdido')).toBeDefined();
    expect(boton('Dar de baja')).toBeDefined();
    expect(boton('Pagado')).toBeUndefined();
  });

  it('marcar perdido un gafete de contratista pide el contratista y manda su id', async () => {
    const { fixture, raiz, pulsar, portador, enviarFormulario, cerrar } = await crear(gafete());
    await pulsar('Marcar perdido');

    expect(raiz.querySelector('mat-label')?.textContent).toContain('Contratista que lo tenía');
    vi.mocked(invoke).mockResolvedValue([{ id: 'c1', cedula: '111111111', nombre: 'ANA SOLANO' }]);
    portador.setValue('ana');
    await fixture.whenStable();
    expect(invoke).toHaveBeenCalledWith('buscar_para_ingreso', { texto: 'ana', limite: 8 });

    portador.setValue({ id: 'c1', etiqueta: 'ANA SOLANO (111111111)' });
    vi.mocked(invoke).mockResolvedValue([
      { campo: 'estado', antes: 'DISPONIBLE', despues: 'PERDIDO' },
    ]);
    await enviarFormulario();

    expect(invoke).toHaveBeenCalledWith('cambiar_gafete', {
      tipo: 'CONTRATISTA',
      numero: 25,
      cambio: {
        cambio: 'PERDIDO',
        portador_contratista_id: 'c1',
        portador_personal_id: null,
        portador_cedula: null,
      },
    });
    expect(cerrar).toHaveBeenCalledWith({
      cambio: 'PERDIDO',
      cambios: [{ campo: 'estado', antes: 'DISPONIBLE', despues: 'PERDIDO' }],
    });
  });

  it('el de una visita pide la cédula de quien lo tenía', async () => {
    const { raiz, pulsar, portador, enviarFormulario } = await crear(
      gafete({ tipo: 'VISITA', numero: 3 }),
    );
    await pulsar('Marcar perdido');

    expect(raiz.querySelector('mat-label')?.textContent).toContain('Cédula de quien lo tenía');
    portador.setValue('3-3333-3333');
    await enviarFormulario();

    expect(invoke).toHaveBeenCalledWith('cambiar_gafete', {
      tipo: 'VISITA',
      numero: 3,
      cambio: {
        cambio: 'PERDIDO',
        portador_contratista_id: null,
        portador_personal_id: null,
        portador_cedula: '3-3333-3333',
      },
    });
  });

  it('el provisional de KOF busca en el personal de KOF', async () => {
    const { fixture, pulsar, portador, enviarFormulario } = await crear(
      gafete({ tipo: 'PROVISIONAL_KOF' }),
    );
    await pulsar('Marcar perdido');

    vi.mocked(invoke).mockResolvedValue([]);
    portador.setValue('luis');
    await fixture.whenStable();
    expect(invoke).toHaveBeenCalledWith('buscar_personal_kof', { texto: 'luis', limite: 8 });

    portador.setValue({ id: 'k1', etiqueta: 'LUIS (12345)' });
    vi.mocked(invoke).mockResolvedValue([]);
    await enviarFormulario();

    expect(invoke).toHaveBeenCalledWith('cambiar_gafete', {
      tipo: 'PROVISIONAL_KOF',
      numero: 25,
      cambio: expect.objectContaining({
        portador_personal_id: 'k1',
        portador_contratista_id: null,
      }),
    });
  });

  it('el rechazo del núcleo sobre el último portador se ve junto a su campo', async () => {
    vi.mocked(invoke).mockRejectedValue({
      tipo: 'negocio',
      codigo: 'gafete_cedula_portador_invalida',
      mensaje: 'La cédula del último portador no es válida',
      campo: 'portador',
    });
    const { raiz, pulsar, portador, enviarFormulario, cerrar } = await crear(
      gafete({ tipo: 'VISITA' }),
    );
    await pulsar('Marcar perdido');

    portador.setValue('abc');
    await enviarFormulario();

    expect(raiz.querySelector('mat-error')?.textContent).toContain('La cédula del último portador');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('dar de baja pide confirmar antes de mandarlo', async () => {
    const { raiz, pulsar, cerrar } = await crear(gafete());

    await pulsar('Dar de baja');
    expect(raiz.textContent).toContain('¿Confirmas?');
    expect(invoke).not.toHaveBeenCalled();

    await pulsar('No');
    expect(raiz.textContent).not.toContain('¿Confirmas?');

    await pulsar('Dar de baja');
    await pulsar('Sí, dar de baja');
    expect(invoke).toHaveBeenCalledWith('cambiar_gafete', {
      tipo: 'CONTRATISTA',
      numero: 25,
      cambio: expect.objectContaining({ cambio: 'DE_BAJA' }),
    });
    expect(cerrar).toHaveBeenCalledWith(expect.objectContaining({ cambio: 'DE_BAJA' }));
  });

  it('uno perdido se resuelve como pagado o como apareció, y muestra quién lo tenía', async () => {
    const { raiz, pulsar, cerrar } = await crear(
      gafete({ estado: 'PERDIDO', asignado_nombre: 'ANA SOLANO', asignado_identidad: '111111111' }),
    );

    expect(raiz.querySelector('.datos')?.textContent).toContain('Lo tenía');
    expect(raiz.querySelector('.datos')?.textContent).toContain('Perdido');
    await pulsar('Pagado');

    expect(invoke).toHaveBeenCalledWith('cambiar_gafete', {
      tipo: 'CONTRATISTA',
      numero: 25,
      cambio: expect.objectContaining({ cambio: 'PAGADO', portador_cedula: null }),
    });
    expect(cerrar).toHaveBeenCalledWith(expect.objectContaining({ cambio: 'PAGADO' }));
  });

  it('un error general del núcleo va arriba de los botones', async () => {
    vi.mocked(invoke).mockRejectedValue({
      tipo: 'negocio',
      codigo: 'gafete_no_perdido',
      mensaje: 'El gafete no está perdido',
      campo: null,
    });
    const { raiz, pulsar, cerrar } = await crear(gafete({ estado: 'PERDIDO' }));

    await pulsar('Apareció');

    expect(raiz.querySelector('.error')?.textContent).toContain('El gafete no está perdido');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('el botón de cerrar y Escape cierran sin cambiar nada', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar } = await crear(gafete(), teclas);

    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(2);
    expect(invoke).not.toHaveBeenCalled();
  });
});
