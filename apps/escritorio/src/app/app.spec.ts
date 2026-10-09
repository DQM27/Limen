import { TestBed } from '@angular/core/testing';
import { provideRouter, Router } from '@angular/router';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { App } from './app';
import { routes } from './app.routes';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

async function crear(dentroDeLaApp: boolean) {
  vi.mocked(isTauri).mockReturnValue(dentroDeLaApp);
  vi.mocked(invoke).mockResolvedValue({ id: 'abc', nombre: 'Operador provisional' });
  TestBed.configureTestingModule({ imports: [App], providers: [provideRouter(routes)] });
  const fixture = TestBed.createComponent(App);
  await fixture.whenStable();
  const raiz = fixture.nativeElement as HTMLElement;
  return { fixture, raiz };
}

describe('carcasa de la app', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(isTauri).mockReset();
  });

  it('ofrece un enlace para saltar al contenido, lo primero que alcanza el teclado', async () => {
    const { raiz } = await crear(true);

    const enlace = raiz.querySelector<HTMLAnchorElement>('a.saltar');
    expect(enlace?.textContent).toContain('Saltar al contenido');
    expect(raiz.querySelector('main#contenido')).not.toBeNull();
  });

  it('la navegación tiene nombre y sólo ofrece lo que ya funciona', async () => {
    const { raiz } = await crear(true);

    expect(raiz.querySelector('nav')?.getAttribute('aria-label')).toBe('Secciones');
    const enlaces = [...raiz.querySelectorAll<HTMLAnchorElement>('nav a[mat-list-item]')];

    expect(enlaces.map((a) => a.textContent?.trim())).toEqual([
      expect.stringContaining('Contratistas'),
    ]);
    // Todo lo que se ve es un enlace real: nada de botones apagados.
    expect(enlaces.every((a) => a.hasAttribute('href'))).toBe(true);
    expect(raiz.querySelector('nav [aria-disabled="true"]')).toBeNull();
  });

  it('marca la página actual para los lectores de pantalla', async () => {
    const { fixture, raiz } = await crear(true);
    await TestBed.inject(Router).navigateByUrl('/contratistas');
    await fixture.whenStable();

    const actual = raiz.querySelector('nav a[aria-current="page"]');
    expect(actual?.textContent).toContain('Contratistas');
  });

  it('muestra al operador de este equipo al pie', async () => {
    const { raiz } = await crear(true);

    expect(raiz.querySelector('.barra-pie')?.textContent).toContain('Operador provisional');
  });

  it('fuera de la app avisa que es modo navegador', async () => {
    const { raiz } = await crear(false);

    expect(raiz.querySelector('.barra-pie')?.textContent).toContain('Modo navegador');
    expect(invoke).not.toHaveBeenCalled();
  });

  it('el botón reduce y expande la barra, y lo comunica con aria-expanded', async () => {
    const { fixture, raiz } = await crear(true);
    const boton = raiz.querySelector<HTMLButtonElement>('button[aria-controls="menu-principal"]');

    expect(boton?.getAttribute('aria-expanded')).toBe('true');
    expect(boton?.getAttribute('aria-label')).toBe('Reducir el menú');

    boton?.click();
    await fixture.whenStable();

    expect(boton?.getAttribute('aria-expanded')).toBe('false');
    expect(boton?.getAttribute('aria-label')).toBe('Expandir el menú');
    expect(raiz.querySelector('.barra.reducida')).not.toBeNull();
    // Reducida, el enlace conserva su nombre accesible aunque no se vea el texto.
    expect(raiz.querySelector('nav a[routerLink], nav a[href]')?.getAttribute('aria-label')).toBe(
      'Contratistas',
    );
  });
});
