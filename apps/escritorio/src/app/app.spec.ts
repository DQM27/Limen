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
  vi.mocked(invoke).mockImplementation((comando: string) =>
    Promise.resolve(
      comando === 'usuario_actual'
        ? { id: 'abc', cedula: '111111111', nombre: 'ANA MORA', debe_cambiar_clave: false }
        : [],
    ),
  );
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

  it('la navegación tiene nombre y sólo Contratistas es un enlace activo', async () => {
    const { raiz } = await crear(true);

    expect(raiz.querySelector('nav')?.getAttribute('aria-label')).toBe('Secciones');
    const enlaces = [...raiz.querySelectorAll<HTMLAnchorElement>('nav a[mat-list-item]')];
    const disponibles = enlaces.filter((a) => a.getAttribute('aria-disabled') !== 'true');
    const proximamente = enlaces.filter((a) => a.getAttribute('aria-disabled') === 'true');

    expect(disponibles.map((a) => a.textContent?.trim())).toEqual([
      expect.stringContaining('Contratistas'),
    ]);
    expect(disponibles.every((a) => a.hasAttribute('href'))).toBe(true);
    expect(proximamente).toHaveLength(6);
    // Lo que no existe aún se anuncia como tal y no recibe el foco del teclado
    // (un enlace sin `href` no entra en el orden de tabulación).
    expect(proximamente.every((a) => a.getAttribute('aria-label')?.endsWith('próximamente'))).toBe(
      true,
    );
    expect(proximamente.some((a) => a.hasAttribute('href'))).toBe(false);
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

    expect(raiz.querySelector('.barra-pie')?.textContent).toContain('ANA MORA');
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
    // El contenido y la barra comparten fila: el contenido ocupa lo que deja la barra.
    expect(raiz.querySelector('.carcasa > .barra + main#contenido')).not.toBeNull();
    // Reducida, el enlace conserva su nombre accesible aunque no se vea el texto.
    expect(raiz.querySelector('nav a[routerLink], nav a[href]')?.getAttribute('aria-label')).toBe(
      'Contratistas',
    );
  });

  it('sin sesión muestra la entrada y no la carcasa, con la ventana en modo entrada', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockImplementation((comando: string) =>
      Promise.resolve(comando === 'usuario_actual' ? null : undefined),
    );
    TestBed.configureTestingModule({ imports: [App], providers: [provideRouter(routes)] });
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const raiz = fixture.nativeElement as HTMLElement;

    expect(raiz.querySelector('app-pantalla-entrada')).not.toBeNull();
    expect(raiz.querySelector('.carcasa')).toBeNull();
    expect(raiz.querySelector('a.saltar')).toBeNull();
    expect(invoke).toHaveBeenCalledWith('modo_ventana', { modo: 'entrada' });
  });

  it('mientras el núcleo no responde muestra el splash', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));
    TestBed.configureTestingModule({ imports: [App], providers: [provideRouter(routes)] });
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();

    expect((fixture.nativeElement as HTMLElement).querySelector('app-splash')).not.toBeNull();
  });

  it('con sesión la ventana pasa a modo aplicación y se puede cerrar la sesión', async () => {
    const { fixture, raiz } = await crear(true);

    expect(invoke).toHaveBeenCalledWith('modo_ventana', { modo: 'aplicacion' });
    raiz.querySelector<HTMLButtonElement>('button[aria-label="Cerrar sesión"]')?.click();
    await fixture.whenStable();

    expect(invoke).toHaveBeenCalledWith('cerrar_sesion', undefined);
    expect(raiz.querySelector('app-pantalla-entrada')).not.toBeNull();
  });
});
