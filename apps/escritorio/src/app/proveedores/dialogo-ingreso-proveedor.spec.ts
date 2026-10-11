import { TestBed } from '@angular/core/testing';
import { MatDialogRef } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import { DialogoIngresoProveedor } from './dialogo-ingreso-proveedor';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

type Respuesta = unknown | ((argumentos: Record<string, unknown>) => unknown);

/** Simula el núcleo comando por comando; los textos al escribir salen en mayúsculas, como en el dominio. */
function responder(respuestas: Record<string, Respuesta> = {}): void {
  const mayusculas = (a: Record<string, unknown>) => String(a['texto']).toUpperCase();
  const normales: Record<string, Respuesta> = {
    registrar_entrada_proveedor: 'i1',
    buscar_empresas_proveedoras: [],
    nombre_persona_al_escribir: mayusculas,
    nombre_empresa_al_escribir: mayusculas,
    placa_al_escribir: mayusculas,
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
  const fixture = TestBed.createComponent(DialogoIngresoProveedor);
  fixture.detectChanges();
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const componente = fixture.componentInstance;
  const refrescar = async () => {
    await new Promise((resolver) => setTimeout(resolver, 0));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  /** Llena lo mínimo para que el formulario sea válido. */
  const llenar = () => {
    componente.cedula.setValue('111111111');
    componente.nombre.setValue('ANA SOLANO');
    componente.empresa.setValue({ id: 'p1', nombre: 'LIMPIEZAS DEL SUR' });
    componente.gafete.setValue('7');
  };
  const enviar = async () => {
    raiz.querySelector('form')?.dispatchEvent(new Event('submit', { cancelable: true }));
    await refrescar();
  };
  return { fixture, raiz, cerrar, componente, llenar, enviar, refrescar };
}

describe('modal de ingreso de proveedor', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
  });

  it('pide cédula, nombre, empresa proveedora, medio y gafete', async () => {
    const { raiz } = await crear();

    expect(raiz.querySelector('h2')?.textContent).toContain('Registrar ingreso de proveedor');
    const etiquetas = [...raiz.querySelectorAll('mat-label')].map((e) => e.textContent?.trim());
    expect(etiquetas).toEqual(['Cédula', 'Nombre', 'Empresa proveedora', 'Número de gafete']);
    expect(raiz.querySelector('mat-button-toggle-group.medio')).not.toBeNull();
  });

  it('registra con lo escrito y se cierra con el nombre', async () => {
    const { llenar, enviar, cerrar } = await crear();
    llenar();

    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_entrada_proveedor', {
      entrada: {
        cedula: '111111111',
        nombre: 'ANA SOLANO',
        empresa_id: 'p1',
        medio: 'A_PIE',
        placa: null,
        gafete: 7,
      },
    });
    expect(cerrar).toHaveBeenCalledWith({ nombre: 'ANA SOLANO' });
  });

  it('en vehículo manda la placa; a pie no, aunque se haya escrito antes', async () => {
    const { llenar, enviar, componente, refrescar, raiz } = await crear();
    llenar();
    componente.placa.setValue('ABC-123');

    await enviar();
    expect(invoke).toHaveBeenLastCalledWith('registrar_entrada_proveedor', {
      entrada: expect.objectContaining({ medio: 'A_PIE', placa: null }),
    });

    componente.medio.setValue('VEHICULO');
    await refrescar();
    expect(raiz.textContent).toContain('Placa');
    await enviar();
    expect(invoke).toHaveBeenLastCalledWith('registrar_entrada_proveedor', {
      entrada: expect.objectContaining({ medio: 'VEHICULO', placa: 'ABC-123' }),
    });
  });

  it('un gafete vacío se manda como 0 y el núcleo dice que no vale', async () => {
    responder({
      registrar_entrada_proveedor: errorDelNucleo(
        'gafete_no_registrado',
        'El gafete no existe',
        'gafete',
      ),
    });
    const { llenar, enviar, componente, raiz, cerrar } = await crear();
    llenar();
    componente.gafete.setValue('');

    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_entrada_proveedor', {
      entrada: expect.objectContaining({ gafete: 0 }),
    });
    expect(raiz.querySelector('mat-error')?.textContent).toContain('El gafete no existe');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('cada rechazo del núcleo sale junto a su campo', async () => {
    const casos: [string, string][] = [
      ['cedula', 'Cédula'],
      ['nombre', 'Nombre'],
      ['empresa_id', 'Empresa proveedora'],
    ];
    for (const [campo, etiqueta] of casos) {
      TestBed.resetTestingModule();
      vi.mocked(invoke).mockReset();
      responder({
        registrar_entrada_proveedor: errorDelNucleo('x', `Falla en ${campo}`, campo),
      });
      const { llenar, enviar, raiz } = await crear();
      llenar();

      await enviar();

      const campoConError = [...raiz.querySelectorAll('mat-form-field')].find((f) =>
        f.querySelector('mat-error'),
      );
      expect(campoConError?.querySelector('mat-label')?.textContent?.trim(), campo).toBe(etiqueta);
      expect(campoConError?.querySelector('mat-error')?.textContent).toContain(`Falla en ${campo}`);
    }
  });

  it('un rechazo sin campo (por ejemplo, ya está adentro) va arriba de los botones', async () => {
    responder({
      registrar_entrada_proveedor: errorDelNucleo('ya_esta_adentro', 'Ya está adentro', null),
    });
    const { llenar, enviar, raiz, cerrar } = await crear();
    llenar();

    await enviar();

    expect(raiz.querySelector('.error')?.textContent).toContain('Ya está adentro');
    expect(cerrar).not.toHaveBeenCalled();
  });

  it('el nombre se ve en mayúsculas mientras se escribe', async () => {
    const { raiz, componente, refrescar } = await crear();
    const campo = [...raiz.querySelectorAll<HTMLInputElement>('input')].find((i) =>
      i.closest('mat-form-field')?.textContent?.includes('Nombre'),
    );
    if (!campo) {
      throw new Error('no hay campo de nombre');
    }

    campo.value = 'ana solano';
    campo.dispatchEvent(new Event('input'));
    await refrescar();

    expect(invoke).toHaveBeenCalledWith('nombre_persona_al_escribir', { texto: 'ana solano' });
    expect(componente.nombre.value).toBe('ANA SOLANO');
  });

  it('la empresa se busca en el catálogo de proveedoras y, escrita completa, se toma de la lista', async () => {
    responder({
      buscar_empresas_proveedoras: [{ id: 'p1', nombre: 'LIMPIEZAS DEL SUR' }],
    });
    const { llenar, enviar, componente, refrescar } = await crear();
    llenar();

    componente.empresa.setValue('limpiezas del sur');
    await refrescar();
    expect(invoke).toHaveBeenCalledWith('buscar_empresas_proveedoras', {
      texto: 'limpiezas del sur',
      limite: 8,
    });
    expect(invoke).not.toHaveBeenCalledWith('buscar_empresas', expect.anything());
    await enviar();

    expect(invoke).toHaveBeenCalledWith('registrar_entrada_proveedor', {
      entrada: expect.objectContaining({ empresa_id: 'p1' }),
    });
  });

  it('"Cancelar", el botón de cerrar y Escape cierran sin registrar nada', async () => {
    const teclas = new Subject<KeyboardEvent>();
    const { raiz, cerrar } = await crear(teclas);

    [...raiz.querySelectorAll('button')].find((b) => b.textContent?.includes('Cancelar'))?.click();
    raiz.querySelector<HTMLButtonElement>('button.cerrar')?.click();
    teclas.next(new KeyboardEvent('keydown', { key: 'Escape' }));

    expect(cerrar).toHaveBeenCalledTimes(3);
    expect(invoke).not.toHaveBeenCalledWith('registrar_entrada_proveedor', expect.anything());
  });
});
