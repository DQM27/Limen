import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import type { CandidatoIngreso } from '../nucleo/tipos';
import { DialogoIngreso } from './dialogo-ingreso';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

function candidato(cambios: Partial<CandidatoIngreso> = {}): CandidatoIngreso {
  return {
    id: 'c1',
    cedula: '111111111',
    nombre: 'ANA SOLANO',
    empresa_id: 'e1',
    empresa_nombre: 'ACME S.A.',
    tipo_ingreso: 'PRAIND',
    fecha_vencimiento_praind: '2099-01-01',
    tiene_acceso: true,
    requiere_gafete: true,
    puede_entrar: true,
    acceso: { resultado: 'PERMITIDO', dias_para_vencer: null, motivo: null },
    motivo: null,
    adentro_por: null,
    gafetes_perdidos: [],
    ...cambios,
  };
}

type Respuesta = unknown | ((argumentos: Record<string, unknown>) => unknown);

/** Simula el núcleo comando por comando; la placa al escribir sale en mayúsculas, como en el dominio. */
function responder(respuestas: Record<string, Respuesta> = {}): void {
  const normales: Record<string, Respuesta> = {
    buscar_para_ingreso: [],
    preparar_ingreso: candidato(),
    registrar_entrada_contratista: {
      ingreso_id: 'i1',
      acceso: { resultado: 'PERMITIDO', dias_para_vencer: null, motivo: null },
    },
    placa_al_escribir: (a: Record<string, unknown>) => String(a['texto']).toUpperCase(),
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

async function crear(
  datos: { contratistaId?: string } = {},
  teclas = new Subject<KeyboardEvent>(),
) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
      { provide: MAT_DIALOG_DATA, useValue: datos },
    ],
  });
  const fixture = TestBed.createComponent(DialogoIngreso);
  await fixture.whenStable();
  fixture.detectChanges();
  const raiz = fixture.nativeElement as HTMLElement;
  const boton = (texto: string) =>
    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.trim() === texto);
  const refrescar = async () => {
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
  const elegirPrimero = async () => {
    await vi.waitFor(() => expect(opciones().length).toBeGreaterThan(0));
    opciones()[0]?.click();
    await refrescar();
  };
  const enviar = async () => {
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await refrescar();
  };
  return {
    fixture,
    raiz,
    cerrar,
    boton,
    refrescar,
    escribirBusqueda,
    opciones,
    elegirPrimero,
    enviar,
    componente: fixture.componentInstance,
  };
}

describe('modal de ingreso de contratista', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
    document.querySelectorAll('.cdk-overlay-container > *').forEach((e) => e.remove());
  });

  it('empieza por buscar y no se puede registrar todavía', async () => {
    const { raiz, boton } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Registrar ingreso');
    expect(raiz.querySelector('mat-label')?.textContent).toContain('Cédula o nombre');
    expect(boton('Registrar ingreso')?.hasAttribute('disabled')).toBe(true);
  });

  it('al escribir muestra a quien coincide, con lo que dijo el núcleo sobre su acceso', async () => {
    responder({
      buscar_para_ingreso: [
        candidato(),
        candidato({
          id: 'c2',
          nombre: 'BETO MORA',
          puede_entrar: false,
          acceso: null,
          motivo: { codigo: 'praind_vencido', mensaje: 'PRAIND vencido' },
        }),
        candidato({ id: 'c3', nombre: 'CARLA', adentro_por: 'CONTRATISTA', puede_entrar: false }),
      ],
    });
    const { escribirBusqueda, opciones } = await crear();

    await escribirBusqueda('a');

    expect(invoke).toHaveBeenCalledWith('buscar_para_ingreso', { texto: 'a', limite: 5 });
    await vi.waitFor(() => expect(opciones()).toHaveLength(3));
    const filas = opciones().map((b) => ({
      nombre: b.querySelector('.nombre')?.textContent?.trim(),
      estado: b.querySelector('.estado')?.textContent?.trim(),
      malo: b.querySelector('.estado')?.classList.contains('malo'),
    }));
    expect(filas).toEqual([
      { nombre: 'ANA SOLANO', estado: 'Vigente', malo: false },
      { nombre: 'BETO MORA', estado: 'PRAIND vencido', malo: true },
      { nombre: 'CARLA', estado: 'Ya está adentro', malo: true },
    ]);
  });

  it('si no encuentra a nadie, dice cómo crearlo', async () => {
    const { escribirBusqueda } = await crear();

    await escribirBusqueda('zzz');

    await vi.waitFor(() =>
      expect(document.querySelector('mat-option.opcion')?.textContent).toContain(
        'No se encontró a nadie',
      ),
    );
    expect(document.querySelector('mat-option.opcion')?.textContent).toContain('Nuevo contratista');
  });

  it('las coincidencias flotan fuera del modal: no cambian lo que hay dentro de él', async () => {
    responder({ buscar_para_ingreso: [candidato(), candidato({ id: 'c2', nombre: 'OTRO' })] });
    const { raiz, escribirBusqueda, opciones } = await crear();
    const antes = raiz.innerHTML;

    await escribirBusqueda('a');
    await vi.waitFor(() => expect(opciones()).toHaveLength(2));

    expect(opciones().some((o) => raiz.contains(o))).toBe(false);
    expect(raiz.querySelector('.resultados, .resultado')).toBeNull();
    // Lo único que cambia dentro del modal es el texto del campo y el estado del autocompletar.
    expect(raiz.querySelectorAll('mat-form-field')).toHaveLength(1);
    expect(antes).toContain('Escribe la cédula o el nombre');
    expect(raiz.textContent).toContain('Escribe la cédula o el nombre');
  });

  it('elegir a alguien que puede entrar muestra su ficha y el formulario del ingreso', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { raiz, escribirBusqueda, elegirPrimero, boton } = await crear();

    await escribirBusqueda('ana');
    await elegirPrimero();

    expect(raiz.querySelector('.ficha .nombre')?.textContent).toContain('ANA SOLANO');
    expect(raiz.querySelector('.ficha')?.textContent).toContain('111111111');
    expect(raiz.querySelector('.ficha')?.textContent).toContain('ACME S.A.');
    expect(raiz.querySelector('.ficha')?.textContent).toContain('PRAIND');
    expect(raiz.querySelector('mat-button-toggle-group.medio')).not.toBeNull();
    expect(raiz.textContent).toContain('Número de gafete');
    expect(boton('Registrar ingreso')?.hasAttribute('disabled')).toBe(false);
    expect(boton('Cambiar')).toBeDefined();
  });

  it('"Cambiar" vuelve al buscador', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { raiz, escribirBusqueda, elegirPrimero, boton, refrescar } = await crear();
    await escribirBusqueda('ana');
    await elegirPrimero();

    boton('Cambiar')?.click();
    await refrescar();

    expect(raiz.querySelector('.ficha')).toBeNull();
    expect(raiz.querySelector('mat-label')?.textContent).toContain('Cédula o nombre');
  });

  it('Enter en el buscador elige el primer resultado', async () => {
    responder({ buscar_para_ingreso: [candidato(), candidato({ id: 'c2', nombre: 'OTRO' })] });
    const { raiz, fixture, refrescar } = await crear();
    const campo = raiz.querySelector<HTMLInputElement>('input');
    if (!campo) {
      throw new Error('no hay buscador');
    }
    campo.value = '111111111';
    campo.dispatchEvent(new Event('input'));

    campo.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', cancelable: true }));
    await refrescar();
    await fixture.whenStable();
    fixture.detectChanges();

    expect(raiz.querySelector('.ficha .nombre')?.textContent).toContain('ANA SOLANO');
  });

  it('registra el ingreso con un gafete y se cierra con la persona y su acceso', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { raiz, escribirBusqueda, elegirPrimero, componente, enviar, cerrar } = await crear();
    await escribirBusqueda('ana');
    await elegirPrimero();

    const campo = [...raiz.querySelectorAll<HTMLInputElement>('input')].find(
      (i) => i.getAttribute('inputmode') === 'numeric',
    );
    if (!campo) {
      throw new Error('no hay campo de gafete');
    }
    campo.value = '25';
    campo.dispatchEvent(new Event('input'));
    await enviar();

    expect(componente.gafete.value).toBe('25');
    expect(invoke).toHaveBeenCalledWith('registrar_entrada_contratista', {
      entrada: {
        contratista_id: 'c1',
        medio: 'A_PIE',
        placa: null,
        gafete: 25,
        sin_gafete: false,
      },
    });
    expect(cerrar).toHaveBeenCalledWith({
      nombre: 'ANA SOLANO',
      acceso: { resultado: 'PERMITIDO', dias_para_vencer: null, motivo: null },
    });
  });

  it('"Sin gafete" y un número son una cosa o la otra', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { escribirBusqueda, elegirPrimero, componente, enviar, fixture } = await crear();
    await escribirBusqueda('ana');
    await elegirPrimero();
    componente.gafete.setValue('25');

    componente['alMarcarSinGafete'](true);
    expect(componente.gafete.value, 'marcar S/G borra el número').toBe('');
    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_entrada_contratista', {
      entrada: {
        contratista_id: 'c1',
        medio: 'A_PIE',
        placa: null,
        gafete: null,
        sin_gafete: true,
      },
    });
    fixture.destroy();
  });

  it('el botón S/G se enciende y se apaga: encendido deshabilita y borra el número', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { raiz, escribirBusqueda, elegirPrimero, componente, refrescar } = await crear();
    await escribirBusqueda('ana');
    await elegirPrimero();
    const numero = () =>
      [...raiz.querySelectorAll<HTMLInputElement>('input')].find(
        (i) => i.getAttribute('inputmode') === 'numeric',
      );
    componente.gafete.setValue('25');
    const boton = raiz.querySelector<HTMLButtonElement>('mat-button-toggle.sin-gafete button');
    expect(boton?.textContent?.trim()).toBe('S/G');
    expect(boton?.getAttribute('aria-pressed')).toBe('false');
    expect(numero()?.disabled).toBe(false);

    boton?.click();
    await refrescar();

    expect(boton?.getAttribute('aria-pressed')).toBe('true');
    expect(componente.sinGafete.value).toBe(true);
    expect(componente.gafete.value, 'encendido borra el número').toBe('');
    expect(numero()?.disabled, 'y ya no se puede escribir').toBe(true);

    boton?.click();
    await refrescar();

    expect(componente.sinGafete.value).toBe(false);
    expect(numero()?.disabled, 'apagado lo vuelve a habilitar').toBe(false);
  });

  it('con S/G encendido, el ingreso se manda sin gafete', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { raiz, escribirBusqueda, elegirPrimero, refrescar, enviar } = await crear();
    await escribirBusqueda('ana');
    await elegirPrimero();

    raiz.querySelector<HTMLButtonElement>('mat-button-toggle.sin-gafete button')?.click();
    await refrescar();
    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_entrada_contratista', {
      entrada: expect.objectContaining({ gafete: null, sin_gafete: true }),
    });
  });

  it('en vehículo pide la placa, que se ve en mayúsculas al escribirla, y la manda', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { raiz, escribirBusqueda, elegirPrimero, componente, refrescar, enviar } = await crear();
    await escribirBusqueda('ana');
    await elegirPrimero();
    expect(raiz.textContent).not.toContain('Placa');

    componente.medio.setValue('VEHICULO');
    await refrescar();
    expect(raiz.textContent).toContain('Placa');

    const campo = [...raiz.querySelectorAll<HTMLInputElement>('input')].find((i) =>
      i.closest('mat-form-field')?.textContent?.includes('Placa'),
    );
    if (!campo) {
      throw new Error('no hay campo de placa');
    }
    campo.value = 'abc-123';
    campo.dispatchEvent(new Event('input'));
    await refrescar();
    expect(invoke).toHaveBeenCalledWith('placa_al_escribir', { texto: 'abc-123' });
    expect(componente.placa.value).toBe('ABC-123');

    componente.sinGafete.setValue(true);
    await enviar();
    expect(invoke).toHaveBeenCalledWith('registrar_entrada_contratista', {
      entrada: expect.objectContaining({ medio: 'VEHICULO', placa: 'ABC-123' }),
    });
  });

  it('a pie no manda placa aunque se haya escrito antes', async () => {
    responder({ buscar_para_ingreso: [candidato()] });
    const { escribirBusqueda, elegirPrimero, componente, enviar } = await crear();
    await escribirBusqueda('ana');
    await elegirPrimero();
    componente.placa.setValue('ABC-123');
    componente.sinGafete.setValue(true);

    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_entrada_contratista', {
      entrada: expect.objectContaining({ medio: 'A_PIE', placa: null }),
    });
  });

  it('IN HOUSE no lleva gafete: no se pide y se avisa por qué', async () => {
    responder({
      buscar_para_ingreso: [candidato({ tipo_ingreso: 'IN_HOUSE', requiere_gafete: false })],
    });
    const { raiz, escribirBusqueda, elegirPrimero } = await crear();

    await escribirBusqueda('ana');
    await elegirPrimero();

    expect(raiz.textContent).not.toContain('Número de gafete');
    expect(raiz.textContent).toContain('su credencial ya es el gafete');
  });

  it('si el núcleo no lo deja entrar, muestra el motivo y no ofrece registrar', async () => {
    responder({
      buscar_para_ingreso: [
        candidato({
          puede_entrar: false,
          acceso: null,
          motivo: { codigo: 'praind_vencido', mensaje: 'El PRAIND está vencido' },
        }),
      ],
    });
    const { raiz, escribirBusqueda, elegirPrimero, boton } = await crear();

    await escribirBusqueda('ana');
    await elegirPrimero();

    expect(raiz.querySelector('.caja.error')?.textContent).toContain('El PRAIND está vencido');
    expect(raiz.querySelector('mat-button-toggle-group.medio')).toBeNull();
    expect(boton('Registrar ingreso')?.hasAttribute('disabled')).toBe(true);
  });

  it('si ya está adentro, lo dice y manda a registrar su salida en Dentro', async () => {
    responder({
      buscar_para_ingreso: [candidato({ adentro_por: 'PROVEEDOR', puede_entrar: false })],
    });
    const { raiz, escribirBusqueda, elegirPrimero } = await crear();

    await escribirBusqueda('ana');
    await elegirPrimero();

    expect(raiz.querySelector('.caja.info')?.textContent).toContain(
      'Ya está adentro por Proveedor',
    );
    expect(raiz.querySelector('.caja.info')?.textContent).toContain('«Dentro»');
  });

  it('avisa si el PRAIND está por vencer y si tiene gafetes perdidos, sin impedir nada', async () => {
    responder({
      buscar_para_ingreso: [
        candidato({
          acceso: { resultado: 'PERMITIDO_CON_ADVERTENCIA', dias_para_vencer: 11, motivo: null },
          gafetes_perdidos: [4, 9],
        }),
      ],
    });
    const { raiz, escribirBusqueda, elegirPrimero, boton } = await crear();

    await escribirBusqueda('ana');
    await elegirPrimero();

    expect(raiz.querySelector('.caja.advertencia')?.textContent).toContain('Por vencer (11 días)');
    expect(raiz.textContent).toContain('gafetes perdidos a su nombre: 4, 9');
    expect(boton('Registrar ingreso')?.hasAttribute('disabled')).toBe(false);
  });

  it('abierto en una persona, pide su ficha y no ofrece volver a buscar', async () => {
    const { raiz, boton } = await crear({ contratistaId: 'c1' });

    expect(invoke).toHaveBeenCalledWith('preparar_ingreso', { contratistaId: 'c1' });
    expect(raiz.querySelector('.ficha .nombre')?.textContent).toContain('ANA SOLANO');
    expect(boton('Cambiar')).toBeUndefined();
  });

  it('si la ficha no se puede abrir, muestra el mensaje del núcleo', async () => {
    responder({ preparar_ingreso: errorDelNucleo('no_encontrado', 'No se encontró', null) });

    const { raiz } = await crear({ contratistaId: 'x' });

    expect(raiz.querySelector('.error')?.textContent).toContain('No se encontró');
  });

  it('el rechazo del gafete se ve junto a su campo y no se cierra', async () => {
    responder({
      registrar_entrada_contratista: errorDelNucleo(
        'gafete_requerido',
        'Indica un gafete o «Sin gafete»',
        'gafete',
      ),
    });
    const { raiz, cerrar, enviar } = await crear({ contratistaId: 'c1' });

    await enviar();

    expect(raiz.querySelector('mat-error')?.textContent).toContain('Indica un gafete');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('un rechazo sin campo (por ejemplo, un choque con otro equipo) va arriba de los botones', async () => {
    responder({
      registrar_entrada_contratista: errorDelNucleo('ya_esta_adentro', 'Ya está adentro', null),
    });
    const { raiz, cerrar, enviar } = await crear({ contratistaId: 'c1' });

    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Ya está adentro');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('"Cancelar", el botón de cerrar y Escape cierran sin registrar nada', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar, boton } = await crear({}, teclas);

    boton('Cancelar')?.click();
    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(3);
    expect(invoke).not.toHaveBeenCalledWith('registrar_entrada_contratista', expect.anything());
  });
});
