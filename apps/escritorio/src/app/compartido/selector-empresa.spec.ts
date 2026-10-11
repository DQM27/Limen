import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { FormControl } from '@angular/forms';
import { MatDialog } from '@angular/material/dialog';
import { invoke } from '@tauri-apps/api/core';
import type { CatalogoDeEmpresas } from './dialogo-empresa';
import type { Empresa } from '../nucleo/tipos';
import { SelectorEmpresa } from './selector-empresa';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

@Component({
  imports: [SelectorEmpresa],
  template: `<app-selector-empresa
    [control]="control"
    [catalogo]="catalogo()"
    etiqueta="Empresa"
  />`,
})
class Anfitrion {
  control = new FormControl<string | Empresa>('', { nonNullable: true });
  catalogo = signal<CatalogoDeEmpresas>('contratistas');
}

type Respuesta = unknown | ((argumentos: Record<string, unknown>) => unknown);

function responder(respuestas: Record<string, Respuesta> = {}): void {
  vi.mocked(invoke).mockImplementation((comando: string, argumentos?: unknown) => {
    const normales: Record<string, Respuesta> = {
      nombre_empresa_al_escribir: (a: Record<string, unknown>) => String(a['texto']).toUpperCase(),
    };
    const respuesta = comando in respuestas ? respuestas[comando] : normales[comando];
    return Promise.resolve(
      typeof respuesta === 'function'
        ? respuesta((argumentos ?? {}) as Record<string, unknown>)
        : (respuesta ?? []),
    );
  });
}

async function crear(catalogo: CatalogoDeEmpresas = 'contratistas') {
  const fixture = TestBed.createComponent(Anfitrion);
  fixture.componentInstance.catalogo.set(catalogo);
  fixture.detectChanges();
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  const refrescar = async () => {
    await new Promise((resolver) => setTimeout(resolver, 0));
    await fixture.whenStable();
    fixture.detectChanges();
  };
  const escribir = async (texto: string) => {
    const campo = raiz.querySelector<HTMLInputElement>('input');
    if (!campo) {
      throw new Error('no hay campo');
    }
    campo.focus();
    campo.value = texto;
    campo.dispatchEvent(new Event('input'));
    await refrescar();
  };
  const opciones = () => [...document.querySelectorAll<HTMLElement>('mat-option')];
  return { fixture, raiz, anfitrion: fixture.componentInstance, refrescar, escribir, opciones };
}

describe('selector de empresa', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    responder();
    document.querySelectorAll('.cdk-overlay-container > *').forEach((e) => e.remove());
  });

  it('busca en el catálogo de contratistas por omisión', async () => {
    responder({ buscar_empresas: [{ id: 'e1', nombre: 'ACME S.A.' }] });
    const { escribir, opciones } = await crear();

    await escribir('acm');
    await vi.waitFor(() =>
      expect(opciones().map((o) => o.textContent?.trim())).toEqual(['ACME S.A.']),
    );

    expect(invoke).toHaveBeenCalledWith('buscar_empresas', { texto: 'ACM', limite: 8 });
  });

  it('con el catálogo de proveedoras busca en ese', async () => {
    responder({ buscar_empresas_proveedoras: [{ id: 'p1', nombre: 'LIMPIEZAS' }] });
    const { escribir, opciones } = await crear('proveedoras');

    await escribir('lim');
    await vi.waitFor(() => expect(opciones()).toHaveLength(1));

    expect(invoke).toHaveBeenCalledWith('buscar_empresas_proveedoras', { texto: 'LIM', limite: 8 });
    expect(invoke).not.toHaveBeenCalledWith('buscar_empresas', expect.anything());
  });

  it('elegir una de la lista deja a la empresa en el control', async () => {
    responder({ buscar_empresas: [{ id: 'e1', nombre: 'ACME S.A.' }] });
    const { escribir, opciones, anfitrion, refrescar } = await crear();
    await escribir('acm');
    await vi.waitFor(() => expect(opciones()).toHaveLength(1));

    opciones()[0]?.click();
    await refrescar();

    expect(anfitrion.control.value).toEqual({ id: 'e1', nombre: 'ACME S.A.' });
  });

  it('si ninguna coincide, avisa que se crea con el botón +', async () => {
    const { escribir, opciones } = await crear();

    await escribir('zzz');

    await vi.waitFor(() => expect(opciones()[0]?.textContent).toContain('usa el botón +'));
    expect(opciones()[0]?.getAttribute('aria-disabled')).toBe('true');
  });

  it('lo que se escribe se ve en mayúsculas', async () => {
    const { escribir, anfitrion } = await crear();

    await escribir('acme');

    expect(anfitrion.control.value).toBe('ACME');
  });

  it('idElegido: la elegida, la escrita completa que coincide, o vacío', async () => {
    responder({ buscar_empresas: [{ id: 'e1', nombre: 'ACME S.A.' }] });
    const { fixture, escribir, opciones, anfitrion } = await crear();
    const selector = fixture.debugElement.children[0]?.componentInstance as SelectorEmpresa;

    expect(selector.idElegido(), 'sin nada').toBe('');

    await escribir('acme s.a.');
    await vi.waitFor(() => expect(opciones()).toHaveLength(1));
    expect(selector.idElegido(), 'escrita completa').toBe('e1');

    anfitrion.control.setValue('otra cosa', { emitEvent: false });
    expect(selector.idElegido(), 'escrita que no coincide').toBe('');

    anfitrion.control.setValue({ id: 'e9', nombre: 'ELEGIDA' }, { emitEvent: false });
    expect(selector.idElegido(), 'elegida').toBe('e9');
  });

  it('el botón + abre el modal del catálogo que corresponde', async () => {
    const { raiz, fixture } = await crear('proveedoras');

    raiz.querySelector<HTMLButtonElement>('button.nueva')?.click();
    await fixture.whenStable();

    const modal = document.querySelector('app-dialogo-empresa');
    expect(modal?.querySelector('h2')?.textContent).toContain('Nueva empresa proveedora');
    expect(TestBed.inject(MatDialog).openDialogs[0]?.disableClose).toBe(true);
    TestBed.inject(MatDialog).closeAll();
  });

  it('el botón + de contratistas abre "Nueva empresa" a secas', async () => {
    const { raiz, fixture } = await crear('contratistas');

    raiz.querySelector<HTMLButtonElement>('button.nueva')?.click();
    await fixture.whenStable();

    const titulo = document.querySelector('app-dialogo-empresa h2')?.textContent?.trim();
    expect(titulo).toBe('Nueva empresa');
    TestBed.inject(MatDialog).closeAll();
  });

  it('los errores del núcleo puestos en el control se ven junto al campo', async () => {
    const { anfitrion, raiz, refrescar } = await crear();

    anfitrion.control.setErrors({ nucleo: 'La empresa es obligatoria' });
    anfitrion.control.markAsTouched();
    await refrescar();

    expect(raiz.querySelector('mat-error')?.textContent).toContain('La empresa es obligatoria');
  });
});
