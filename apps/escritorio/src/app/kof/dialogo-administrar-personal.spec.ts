import { TestBed } from '@angular/core/testing';
import { MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import { Avisos } from '../compartido/avisos';
import type { PersonalKof } from '../nucleo/tipos';
import { DialogoAdministrarPersonal } from './dialogo-administrar-personal';

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
    buscar_personal_kof: [ana, luis],
    editar_personal_kof: [],
    nombre_persona_al_escribir: (a: Record<string, unknown>) => String(a['texto']).toUpperCase(),
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

const llamadas = (comando: string) =>
  vi.mocked(invoke).mock.calls.filter(([nombre]) => nombre === comando);

async function crear(teclas = new Subject<KeyboardEvent>()) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
    ],
  });
  const fixture = TestBed.createComponent(DialogoAdministrarPersonal);
  fixture.detectChanges();
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const refrescar = async () => {
    await new Promise((resolver) => setTimeout(resolver, 0));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  const buscar = async (texto: string) => {
    const campo = raiz.querySelector<HTMLInputElement>('input');
    if (!campo) {
      throw new Error('no hay buscador');
    }
    campo.value = texto;
    campo.dispatchEvent(new Event('input'));
    await refrescar();
    await vi.waitFor(() => expect(llamadas('buscar_personal_kof').length).toBeGreaterThan(0));
    await refrescar();
  };
  const boton = (texto: string) =>
    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes(texto));
  return {
    fixture,
    raiz,
    cerrar,
    teclas,
    refrescar,
    buscar,
    boton,
    componente: fixture.componentInstance,
  };
}

describe('modal para administrar al personal KOF', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
  });

  it('antes de buscar, invita a escribir', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Personal KOF');
    expect(raiz.querySelector('.vacio')?.textContent).toContain('Escribe un código o un nombre');
  });

  it('muestra a quien coincide con su código; el inactivo lo dice y ofrece activarlo', async () => {
    const { raiz, buscar } = await crear();

    await buscar('a');

    expect(invoke).toHaveBeenCalledWith('buscar_personal_kof', { texto: 'a', limite: 6 });
    const filas = [...raiz.querySelectorAll('.persona')];
    expect(filas.map((f) => f.querySelector('.nombre')?.textContent)).toEqual([
      'ANA MORA',
      'LUIS RUIZ',
    ]);
    expect(filas[0]?.textContent).toContain('5040017');
    expect(filas[0]?.querySelector('.estado')).toBeNull();
    expect(filas[1]?.querySelector('.estado')?.textContent).toContain('Inactivo');
    expect(filas[0]?.querySelector('.activo')?.textContent).toContain('Desactivar');
    expect(filas[1]?.querySelector('.activo')?.textContent).toContain('Activar');
  });

  it('si no encuentra a nadie, lo dice', async () => {
    responder({ buscar_personal_kof: [] });
    const { raiz, buscar } = await crear();

    await buscar('zzz');

    expect(raiz.querySelector('.vacio')?.textContent).toContain('No se encontró a nadie');
  });

  it('desactivar manda el mismo nombre con activo en falso, avisa y lo marca inactivo', async () => {
    const { raiz, buscar, refrescar } = await crear();
    const exito = vi.spyOn(TestBed.inject(Avisos), 'exito');
    await buscar('a');

    raiz.querySelector<HTMLButtonElement>('.persona .activo')?.click();
    await refrescar();

    expect(invoke).toHaveBeenCalledWith('editar_personal_kof', {
      id: 'k1',
      nombre: 'ANA MORA',
      activo: false,
    });
    expect(exito).toHaveBeenCalledWith('ANA MORA', 'Persona desactivada');
    expect(raiz.querySelector('.persona .estado')?.textContent).toContain('Inactivo');
  });

  it('activar a una persona inactiva lo manda con activo en verdadero', async () => {
    const { raiz, buscar, refrescar } = await crear();
    const exito = vi.spyOn(TestBed.inject(Avisos), 'exito');
    await buscar('a');

    raiz.querySelectorAll<HTMLButtonElement>('.persona .activo')[1]?.click();
    await refrescar();

    expect(invoke).toHaveBeenCalledWith('editar_personal_kof', {
      id: 'k2',
      nombre: 'LUIS RUIZ',
      activo: true,
    });
    expect(exito).toHaveBeenCalledWith('LUIS RUIZ', 'Persona activada');
  });

  it('corregir el nombre lo muestra en mayúsculas al escribir, lo guarda y lo actualiza', async () => {
    const { raiz, buscar, refrescar, componente } = await crear();
    await buscar('a');

    raiz.querySelector<HTMLButtonElement>('.persona .corregir')?.click();
    await refrescar();
    const campo = raiz.querySelector<HTMLInputElement>('.correccion input');
    if (!campo) {
      throw new Error('no hay campo de corrección');
    }
    expect(componente.nombre.value).toBe('ANA MORA');
    campo.value = 'ana mora diaz';
    campo.dispatchEvent(new Event('input'));
    await refrescar();
    expect(componente.nombre.value).toBe('ANA MORA DIAZ');
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await refrescar();

    expect(invoke).toHaveBeenCalledWith('editar_personal_kof', {
      id: 'k1',
      nombre: 'ANA MORA DIAZ',
      activo: true,
    });
    expect(raiz.querySelector('.correccion')).toBeNull();
    expect(raiz.querySelector('.persona .nombre')?.textContent).toBe('ANA MORA DIAZ');
  });

  it('el rechazo del nombre sale junto al campo y no cierra la corrección', async () => {
    responder({
      editar_personal_kof: {
        tipo: 'negocio',
        codigo: 'nombre_vacio',
        mensaje: 'El nombre es obligatorio',
        campo: 'nombre',
      },
    });
    const { raiz, buscar, refrescar, componente } = await crear();
    await buscar('a');
    raiz.querySelector<HTMLButtonElement>('.persona .corregir')?.click();
    await refrescar();
    componente.nombre.setValue('');

    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await refrescar();

    expect(raiz.querySelector('mat-error')?.textContent).toContain('El nombre es obligatorio');
    expect(raiz.querySelector('.correccion')).not.toBeNull();
  });

  it('un rechazo sin campo (por ejemplo, sin sesión) va arriba de los botones', async () => {
    responder({
      editar_personal_kof: {
        tipo: 'negocio',
        codigo: 'sin_sesion',
        mensaje: 'Inicia sesión',
        campo: null,
      },
    });
    const { raiz, buscar, refrescar } = await crear();
    await buscar('a');

    raiz.querySelector<HTMLButtonElement>('.persona .activo')?.click();
    await refrescar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Inicia sesión');
  });

  it('Escape cancela primero la corrección y luego cierra; «Cerrar» cierra siempre', async () => {
    const { raiz, buscar, refrescar, teclas, cerrar, boton } = await crear();
    await buscar('a');
    raiz.querySelector<HTMLButtonElement>('.persona .corregir')?.click();
    await refrescar();

    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));
    await refrescar();
    expect(raiz.querySelector('.correccion')).toBeNull();
    expect(cerrar).not.toHaveBeenCalled();

    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));
    boton('Cerrar')?.click();
    expect(cerrar).toHaveBeenCalledTimes(2);
  });
});
