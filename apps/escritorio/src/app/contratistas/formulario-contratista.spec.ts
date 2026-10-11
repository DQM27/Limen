import { TestBed } from '@angular/core/testing';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import type { FilaContratista } from '../nucleo/tipos';
import { FormularioContratista } from './formulario-contratista';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const ana: FilaContratista = {
  id: 'c1',
  cedula: '111111111',
  nombre: 'ANA MORA',
  empresa_id: 'e1',
  empresa_nombre: 'ACME S.A.',
  tipo_ingreso: 'PRAIND',
  fecha_vencimiento_praind: '2099-01-01',
  tiene_acceso: true,
  requiere_gafete: true,
  acceso: { resultado: 'PERMITIDO', dias_para_vencer: null, motivo: null },
};

const esperar = (): Promise<void> => new Promise((resolver) => setTimeout(resolver, 20));

function errorDelNucleo(codigo: string, mensaje: string, campo: string | null) {
  return { tipo: 'negocio', codigo, mensaje, campo };
}

type Respuesta = unknown | ((argumentos: Record<string, unknown>) => unknown);

/**
 * Simula el núcleo comando por comando. Lo que no se indica responde lo
 * normal: sin problemas al validar, sin empresas que ofrecer, y el texto en
 * mayúsculas (como lo haría el dominio).
 */
function responder(respuestas: Record<string, Respuesta> = {}): void {
  const normales: Record<string, Respuesta> = {
    validar_contratista: [],
    buscar_empresas: [],
    registrar_contratista: 'nuevo-id',
    editar_contratista: [],
    nombre_persona_al_escribir: (a: Record<string, unknown>) => String(a['texto']).toUpperCase(),
    nombre_empresa_al_escribir: (a: Record<string, unknown>) => String(a['texto']).toUpperCase(),
  };
  vi.mocked(invoke).mockImplementation((comando: string, argumentos?: unknown) => {
    const respuesta = comando in respuestas ? respuestas[comando] : normales[comando];
    const valor =
      typeof respuesta === 'function'
        ? respuesta((argumentos ?? {}) as Record<string, unknown>)
        : respuesta;
    const rechazo =
      valor instanceof Error || (valor as { codigo?: string } | undefined)?.codigo !== undefined;
    return rechazo ? Promise.reject(valor) : Promise.resolve(valor);
  });
}

async function crear(contratista?: FilaContratista, teclas = new Subject<KeyboardEvent>()) {
  const cerrar = vi.fn();
  TestBed.configureTestingModule({
    providers: [
      { provide: MatDialogRef, useValue: { close: cerrar, keydownEvents: () => teclas } },
      { provide: MAT_DIALOG_DATA, useValue: { contratista } },
    ],
  });
  const fixture = TestBed.createComponent(FormularioContratista);
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const enviar = async () => {
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  return { fixture, raiz, cerrar, enviar, form: fixture.componentInstance.form };
}

describe('formulario de contratista', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
  });

  it('para uno nuevo, abre vacío y se titula "Nuevo contratista"', async () => {
    const { raiz, form } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Nuevo contratista');
    expect(raiz.textContent).toContain('Registrar');
    expect(form.getRawValue()).toMatchObject({ cedula: '', nombre: '', tipo: 'PRAIND' });
    expect(form.controls.tieneAcceso.value).toBe(true);
  });

  it('registra con lo escrito, tal cual, y se cierra avisando que registró', async () => {
    const { form, cerrar, enviar } = await crear();

    form.patchValue({
      cedula: ' 222222222 ',
      nombre: 'beto ruiz',
      empresa: { id: 'e1', nombre: 'ACME S.A.' },
      tipo: 'IN_HOUSE',
      fecha: '2099-06-30',
      tieneAcceso: false,
    });
    await enviar();

    const formulario = {
      cedula: ' 222222222 ',
      nombre: 'beto ruiz',
      empresa_id: 'e1',
      tipo_ingreso: 'IN_HOUSE',
      fecha_vencimiento_praind: '2099-06-30',
      tiene_acceso: false,
    };
    expect(invoke).toHaveBeenCalledWith('validar_contratista', {
      id: null,
      contratista: formulario,
    });
    expect(invoke).toHaveBeenCalledWith('registrar_contratista', { contratista: formulario });
    expect(cerrar).toHaveBeenCalledWith({ tipo: 'registrado' });
  });

  it('para editar, precarga a la persona y se titula "Editar contratista"', async () => {
    const { raiz, form } = await crear(ana);

    expect(raiz.querySelector('h2')?.textContent).toContain('Editar contratista');
    expect(raiz.textContent).toContain('Guardar cambios');
    expect(form.getRawValue()).toEqual({
      cedula: '111111111',
      nombre: 'ANA MORA',
      empresa: { id: 'e1', nombre: 'ACME S.A.' },
      tipo: 'PRAIND',
      fecha: '2099-01-01',
      tieneAcceso: true,
    });
  });

  it('al editar, valida y manda el formulario completo con el id y devuelve lo que cambió', async () => {
    const cambios = [{ campo: 'nombre', antes: 'ANA MORA', despues: 'ANA MORA SOTO' }];
    responder({ editar_contratista: cambios });
    const { form, cerrar, enviar } = await crear(ana);

    form.patchValue({ nombre: 'ANA MORA SOTO' });
    await enviar();

    expect(invoke).toHaveBeenCalledWith('validar_contratista', {
      id: 'c1',
      contratista: expect.objectContaining({ nombre: 'ANA MORA SOTO' }),
    });
    expect(invoke).toHaveBeenCalledWith('editar_contratista', {
      id: 'c1',
      contratista: expect.objectContaining({ nombre: 'ANA MORA SOTO', empresa_id: 'e1' }),
    });
    expect(cerrar).toHaveBeenCalledWith({ tipo: 'editado', cambios });
  });

  it('un formulario vacío marca todos los campos a la vez y no guarda nada', async () => {
    responder({
      validar_contratista: [
        errorDelNucleo('cedula_vacia', 'La cédula es obligatoria', 'cedula'),
        errorDelNucleo('nombre_vacio', 'El nombre es obligatorio', 'nombre'),
        errorDelNucleo('empresa_requerida', 'La empresa es obligatoria', 'empresa_id'),
        errorDelNucleo(
          'praind_requerido',
          'La fecha de vencimiento del PRAIND es obligatoria',
          'fecha_vencimiento_praind',
        ),
      ],
    });
    const { raiz, cerrar, enviar } = await crear();

    await enviar();

    const mensajes = [...raiz.querySelectorAll('mat-error')].map((e) => e.textContent?.trim());
    expect(mensajes).toEqual([
      'La cédula es obligatoria',
      'El nombre es obligatorio',
      'La empresa es obligatoria',
      'La fecha de vencimiento del PRAIND es obligatoria',
    ]);
    expect(invoke).not.toHaveBeenCalledWith('registrar_contratista', expect.anything());
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('el cursor va al primer campo con problema', async () => {
    responder({
      validar_contratista: [errorDelNucleo('nombre_vacio', 'El nombre es obligatorio', 'nombre')],
    });
    const { fixture, raiz, enviar } = await crear();
    document.body.append(raiz);

    await enviar();
    await fixture.whenStable();

    expect(document.activeElement?.getAttribute('formControlName')).toBe('nombre');
    raiz.remove();
  });

  it('un problema sin campo va arriba de los botones', async () => {
    responder({
      validar_contratista: [errorDelNucleo('otro', 'Algo general falló.', null)],
    });
    const { raiz, enviar } = await crear();

    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Algo general falló.');
    expect(raiz.querySelector('mat-error')).toBeNull();
  });

  it('si al guardar el núcleo rechaza algo (otro equipo se adelantó), se muestra junto a su campo', async () => {
    responder({
      registrar_contratista: errorDelNucleo(
        'cedula_repetida',
        'Ya existe un contratista con esa cédula',
        'cedula',
      ),
    });
    const { raiz, cerrar, enviar } = await crear();

    await enviar();

    expect(raiz.querySelector('mat-error')?.textContent).toContain('Ya existe un contratista');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('una falla al validar (por ejemplo sin sesión) se muestra y no se guarda', async () => {
    responder({ validar_contratista: errorDelNucleo('sin_sesion', 'Inicia sesión.', null) });
    const { raiz, cerrar, enviar } = await crear();

    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Inicia sesión.');
    expect(invoke).not.toHaveBeenCalledWith('registrar_contratista', expect.anything());
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('el nombre se ve en mayúsculas y sin tildes mientras se escribe, como lo decide el dominio', async () => {
    responder({
      nombre_persona_al_escribir: (a: Record<string, unknown>) =>
        String(a['texto']).toUpperCase().replace('Ó', 'O'),
    });
    const { fixture, raiz, form } = await crear();
    const campo = raiz.querySelector<HTMLInputElement>('input[formControlName=nombre]');
    if (!campo) {
      throw new Error('no hay campo de nombre');
    }

    campo.value = 'jón peña ';
    campo.setSelectionRange(3, 3);
    campo.dispatchEvent(new Event('input'));
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('nombre_persona_al_escribir', { texto: 'jón peña ' });
    expect(form.controls.nombre.value).toBe('JON PEÑA ');
    expect(campo.value).toBe('JON PEÑA ');
    expect(campo.selectionStart).toBe(3);
  });

  it('la empresa que se escribe también se ve en mayúsculas', async () => {
    const { fixture, raiz, form } = await crear();
    const campo = raiz.querySelector<HTMLInputElement>('input[formControlName=empresa]');
    if (!campo) {
      throw new Error('no hay campo de empresa');
    }

    campo.value = 'acme s.a.';
    campo.dispatchEvent(new Event('input'));
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('nombre_empresa_al_escribir', { texto: 'acme s.a.' });
    expect(form.controls.empresa.value).toBe('ACME S.A.');
  });

  it('una empresa escrita completa se toma de la lista aunque no se haya elegido', async () => {
    responder({ buscar_empresas: [{ id: 'e1', nombre: 'ACME S.A.' }] });
    const { fixture, form, enviar } = await crear();

    form.controls.empresa.setValue('acme s.a.');
    await fixture.whenStable();
    form.patchValue({ cedula: '111111111', nombre: 'ANA', fecha: '2099-01-01' });
    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_contratista', {
      contratista: expect.objectContaining({ empresa_id: 'e1' }),
    });
  });

  it('una empresa escrita que no está en la lista va vacía y el núcleo dice que falta', async () => {
    const { form, enviar } = await crear();

    form.patchValue({ empresa: 'algo escrito sin elegir' });
    await enviar();

    expect(invoke).toHaveBeenCalledWith('validar_contratista', {
      id: null,
      contratista: expect.objectContaining({ empresa_id: '' }),
    });
  });

  it('al escribir una empresa ofrece las que trae el núcleo y, si ninguna coincide, avisa dónde crear', async () => {
    responder({ buscar_empresas: [{ id: 'e1', nombre: 'ACME S.A.' }] });
    const { fixture, form } = await crear();
    const interno = fixture.componentInstance as unknown as {
      opciones(): unknown[];
      sinCoincidencias(): boolean;
    };

    form.controls.empresa.setValue('acm');
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('buscar_empresas', { texto: 'acm', limite: 8 });
    expect(interno.opciones()).toEqual([{ id: 'e1', nombre: 'ACME S.A.' }]);
    expect(interno.sinCoincidencias()).toBe(false);

    responder({ buscar_empresas: [] });
    form.controls.empresa.setValue('zzz');
    await fixture.whenStable();

    expect(interno.sinCoincidencias()).toBe(true);
  });

  it('el selector sólo elige: crear una empresa es el botón "+", que abre su modal y deja la nueva elegida', async () => {
    responder({
      registrar_empresa: 'e9',
      buscar_empresas: [{ id: 'e9', nombre: 'NUEVA S.A.' }],
    });
    const { fixture, raiz, form } = await crear();

    raiz.querySelector<HTMLButtonElement>('button.nueva-empresa')?.click();
    await esperar();
    const modal = document.querySelector('app-dialogo-empresa');
    const campo = modal?.querySelector<HTMLInputElement>('input');
    if (!modal || !campo) {
      throw new Error('el modal de empresa no se abrió');
    }
    campo.value = 'nueva s.a.';
    campo.dispatchEvent(new Event('input'));
    await esperar();
    modal.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await esperar();
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('registrar_empresa', { nombre: 'NUEVA S.A.' });
    // El diálogo se cierra con su animación: se espera a que la empresa quede elegida.
    await vi.waitFor(() =>
      expect(form.controls.empresa.value).toEqual({ id: 'e9', nombre: 'NUEVA S.A.' }),
    );
  });

  it('"Cancelar" cierra sin guardar', async () => {
    const { raiz, cerrar } = await crear();

    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes('Cancelar'))?.click();

    expect(cerrar).toHaveBeenCalledWith();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('el botón de cerrar y la tecla Escape cierran sin guardar', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar } = await crear(undefined, teclas);

    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(2);
    expect(invoke).not.toHaveBeenCalled();
  });
});
