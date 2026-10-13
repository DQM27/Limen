import { Component, inject } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { CambiosDeLaNube } from './cambios-de-la-nube';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => false),
}));

let recargas = 0;

@Component({ selector: 'app-prueba', template: '' })
class PantallaDePrueba {
  constructor() {
    inject(CambiosDeLaNube).alLlegar(() => recargas++);
  }
}

describe('cambios de la nube', () => {
  beforeEach(() => {
    recargas = 0;
  });

  it('avisa a las pantallas abiertas y deja de avisar a las que se cerraron', () => {
    const cambios = TestBed.inject(CambiosDeLaNube);
    const pantalla = TestBed.createComponent(PantallaDePrueba);

    cambios.avisar();
    expect(recargas).toBe(1);

    pantalla.destroy();
    cambios.avisar();
    expect(recargas).toBe(1);
  });
});
