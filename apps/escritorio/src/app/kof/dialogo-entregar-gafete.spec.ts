import { TestBed } from '@angular/core/testing';
import { MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import type { PersonalKof } from '../nucleo/tipos';
import { DialogoEntregarGafete } from './dialogo-entregar-gafete';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const ana: PersonalKof = { id: 'k1', codigo_empleado: '5040017', nombre: 'ANA MORA', activo: true };
const luis: PersonalKof = {
  id: 'k2',
  codigo_empleado: '5040018',
  nombre: 'LUIS RUIZ',
  activo: false,
};

type Respuesta = unknown | ((argumentos: Record<string, unknown>) => unknown);

function responder(respuestas: Record<string, Respuesta> = {}): void {
  const normales: Record<string, Respuesta> = {
    buscar_personal_kof: [],
    validar_entrega_gafete_kof: [],
    entregar_gafete_kof: 'p1',
  };
  vi.mocked(invoke).mockImplementation((comando: string, argumentos?: unknown) => {
    const respuesta = comando in respuestas ? respuestas[comando] : normales[comando];
    const valor =
      typeof respuesta === 'function'
        ? respuesta((argumentos ?? {}) as Record<string, unknown>)
        : respuesta;
    const rechazo = (valor as { codigo?: string } | undefined)?.codigo !== undefined;
    return rechazo ? Promise.reject(valor) : Promise.resolve(valor);
  });
}

function errorDelNucleo(codigo: string, mensaje: string, campo: string | null) {
  return { tipo: 'negocio', codigo, mensaje, campo };
}

async function crear(teclas = new Subject<KeyboardEvent>()) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
    ],
  });
  const fixture = TestBed.createComponent(DialogoEntregarGafete);
  fixture.detectChanges();
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const componente = fixture.componentInstance;
  const refrescar = async () => {
    await new Promise((resolver) => setTimeout(resolver, 0));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  const escribirBusqueda = async (texto: string) => {
    const campo = raiz.querySelector<HTMLInputElement>('input');
    if (!campo) {
      throw new Error('no hay buscador');
    }
    // La lista sólo se despliega si el campo tiene el foco, como en uso real.
    campo.focus();
    campo.value = texto;
    campo.dispatchEvent(new Event('input'));
    await refrescar();
  };
  /** Las coincidencias flotan en la capa de encima del modal, no dentro de él. */
  const opciones = () => [...document.querySelectorAll<HTMLElement>('mat-option.opcion')];
  const enviar = async () => {
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await refrescar();
  };
  return { fixture, raiz, cerrar, componente, refrescar, escribirBusqueda, opciones, enviar };
}

describe('modal para entregar el gafete provisional de KOF', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
    document.querySelectorAll('.cdk-overlay-container > *').forEach((e) => e.remove());
  });

  it('pide a la persona y el número del gafete, y explica que es la entrada', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Entregar gafete provisional');
    const etiquetas = [...raiz.querySelectorAll('mat-label')].map((e) => e.textContent?.trim());
    expect(etiquetas).toEqual(['Código o nombre', 'Número de gafete provisional']);
    expect(raiz.textContent).toContain('Entregar el gafete es su entrada');
  });

  it('al escribir ofrece a quien coincide, con su código; el inactivo lo dice', async () => {
    responder({ buscar_personal_kof: [ana, luis] });
    const { escribirBusqueda, opciones } = await crear();

    await escribirBusqueda('a');
    await vi.waitFor(() => expect(opciones()).toHaveLength(2));

    expect(invoke).toHaveBeenCalledWith('buscar_personal_kof', { texto: 'a', limite: 5 });
    expect(opciones().map((o) => o.querySelector('.nombre')?.textContent)).toEqual([
      'ANA MORA',
      'LUIS RUIZ',
    ]);
    expect(opciones()[0]?.textContent).toContain('5040017');
    expect(opciones()[0]?.querySelector('.estado')).toBeNull();
    expect(opciones()[1]?.querySelector('.estado')?.textContent).toContain('Inactivo');
  });

  it('las coincidencias flotan fuera del modal', async () => {
    responder({ buscar_personal_kof: [ana] });
    const { raiz, escribirBusqueda, opciones } = await crear();

    await escribirBusqueda('a');
    await vi.waitFor(() => expect(opciones()).toHaveLength(1));

    expect(opciones().some((o) => raiz.contains(o))).toBe(false);
  });

  it('si no encuentra a nadie, dice cómo darlo de alta', async () => {
    const { escribirBusqueda } = await crear();

    await escribirBusqueda('zzz');

    await vi.waitFor(() =>
      expect(document.querySelector('mat-option.opcion')?.textContent).toContain(
        'No se encontró a nadie',
      ),
    );
    expect(document.querySelector('mat-option.opcion')?.textContent).toContain('Nuevo personal');
  });

  it('entrega el gafete a la persona elegida y se cierra con su nombre', async () => {
    responder({ buscar_personal_kof: [ana] });
    const { escribirBusqueda, opciones, componente, enviar, cerrar, refrescar } = await crear();
    await escribirBusqueda('ana');
    await vi.waitFor(() => expect(opciones()).toHaveLength(1));
    opciones()[0]?.click();
    await refrescar();
    componente.gafete.setValue('3');

    await enviar();

    expect(invoke).toHaveBeenCalledWith('entregar_gafete_kof', { personalId: 'k1', gafete: 3 });
    expect(cerrar).toHaveBeenCalledWith({ nombre: 'ANA MORA' });
  });

  it('elegir a alguien de la lista no vuelve a buscar (el campo recibe a la persona, no un texto)', async () => {
    responder({ buscar_personal_kof: [ana] });
    const { escribirBusqueda, opciones, refrescar } = await crear();
    await escribirBusqueda('ana');
    await vi.waitFor(() => expect(opciones()).toHaveLength(1));

    opciones()[0]?.click();
    await refrescar();

    const busquedas = vi
      .mocked(invoke)
      .mock.calls.filter(([comando]) => comando === 'buscar_personal_kof');
    expect(busquedas).toHaveLength(1);
  });

  it('sin elegir a nadie manda el id vacío y el núcleo dice que hay que elegir', async () => {
    responder({
      entregar_gafete_kof: errorDelNucleo(
        'persona_requerida',
        'Elija a la persona del personal KOF',
        'personal_id',
      ),
    });
    const { componente, enviar, raiz, cerrar } = await crear();
    componente.gafete.setValue('3');

    await enviar();

    expect(invoke).toHaveBeenCalledWith('entregar_gafete_kof', { personalId: '', gafete: 3 });
    expect(raiz.querySelector('mat-error')?.textContent).toContain('Elija a la persona');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('antes de entregar pregunta qué está mal y marca la persona y el gafete a la vez, sin entregar', async () => {
    responder({
      validar_entrega_gafete_kof: [
        errorDelNucleo('persona_requerida', 'Elija a la persona del personal KOF', 'personal_id'),
        errorDelNucleo('gafete_no_registrado', 'El gafete no existe', 'gafete'),
      ],
    });
    const { enviar, raiz, cerrar } = await crear();

    await enviar();

    expect(invoke).toHaveBeenCalledWith('validar_entrega_gafete_kof', {
      personalId: '',
      gafete: 0,
    });
    const mensajes = [...raiz.querySelectorAll('mat-error')].map((e) => e.textContent?.trim());
    expect(mensajes).toEqual(['Elija a la persona del personal KOF', 'El gafete no existe']);
    expect(invoke).not.toHaveBeenCalledWith('entregar_gafete_kof', expect.anything());
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('el rechazo del gafete sale junto a su campo', async () => {
    responder({
      entregar_gafete_kof: errorDelNucleo('gafete_no_registrado', 'El gafete no existe', 'gafete'),
    });
    const { componente, enviar, raiz } = await crear();
    componente.persona.setValue(ana);

    await enviar();

    const campo = [...raiz.querySelectorAll('mat-form-field')].find((f) =>
      f.querySelector('mat-error'),
    );
    expect(campo?.querySelector('mat-label')?.textContent).toContain('gafete provisional');
    expect(campo?.querySelector('mat-error')?.textContent).toContain('El gafete no existe');
  });

  it('una persona inactiva se rechaza junto a su campo', async () => {
    responder({
      entregar_gafete_kof: errorDelNucleo(
        'personal_inactivo',
        'La persona está inactiva',
        'personal_id',
      ),
    });
    const { componente, enviar, raiz } = await crear();
    componente.persona.setValue(luis);
    componente.gafete.setValue('3');

    await enviar();

    expect(raiz.querySelector('mat-error')?.textContent).toContain('La persona está inactiva');
  });

  it('un rechazo sin campo (por ejemplo, ya tiene un gafete) va arriba de los botones', async () => {
    responder({
      entregar_gafete_kof: errorDelNucleo(
        'ya_tiene_prestamo',
        'Ya tiene un gafete provisional',
        null,
      ),
    });
    const { componente, enviar, raiz, cerrar } = await crear();
    componente.persona.setValue(ana);
    componente.gafete.setValue('3');

    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Ya tiene un gafete provisional');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('el gafete sólo admite dígitos', async () => {
    const { raiz, componente, refrescar } = await crear();
    const campo = raiz.querySelector<HTMLInputElement>('input[inputmode="numeric"]');
    if (!campo) {
      throw new Error('no hay campo de gafete');
    }

    campo.value = '1a2';
    campo.dispatchEvent(new Event('input'));
    await refrescar();

    expect(componente.gafete.value).toBe('12');
  });

  it('"Cancelar", el botón de cerrar y Escape cierran sin entregar nada', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar } = await crear(teclas);

    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes('Cancelar'))?.click();
    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(3);
    expect(invoke).not.toHaveBeenCalledWith('entregar_gafete_kof', expect.anything());
  });
});
