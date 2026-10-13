import { Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { Atajo, coincide } from './atajo';

const tecla = (key: string, opciones: KeyboardEventInit = {}) =>
  new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...opciones });

describe('coincide', () => {
  it('reconoce teclas de función y combinaciones con Ctrl (o Cmd)', () => {
    expect(coincide(tecla('F2'), 'F2')).toBe(true);
    expect(coincide(tecla('F3'), 'F2')).toBe(false);
    expect(coincide(tecla('n', { ctrlKey: true }), 'Ctrl+N')).toBe(true);
    expect(coincide(tecla('n', { metaKey: true }), 'Ctrl+N')).toBe(true);
    expect(coincide(tecla('n'), 'Ctrl+N')).toBe(false);
    expect(coincide(tecla('n', { ctrlKey: true, shiftKey: true }), 'Ctrl+N')).toBe(false);
  });

  it('una tecla suelta vale con o sin Mayús (la barra se escribe con Mayús en algunos teclados)', () => {
    expect(coincide(tecla('/'), '/')).toBe(true);
    expect(coincide(tecla('/', { shiftKey: true }), '/')).toBe(true);
    expect(coincide(tecla('/', { ctrlKey: true }), '/')).toBe(false);
  });
});

@Component({
  imports: [Atajo],
  template: `
    <button appAtajo="F2" (click)="pulsado = pulsado + 1">Ingreso</button>
    <button appAtajo="G" disabled (click)="pulsado = pulsado + 100">Apagado</button>
    <input appAtajo="/" />
    <input class="otro" />
  `,
})
class Anfitrion {
  pulsado = 0;
}

function crear() {
  const fixture = TestBed.createComponent(Anfitrion);
  fixture.detectChanges();
  document.body.append(fixture.nativeElement as HTMLElement);
  return { fixture, raiz: fixture.nativeElement as HTMLElement };
}

describe('directiva appAtajo', () => {
  afterEach(() => {
    document.querySelectorAll('.mat-mdc-dialog-container').forEach((e) => e.remove());
  });

  it('pulsa el botón con su tecla y anuncia el atajo a los lectores de pantalla', () => {
    const { fixture, raiz } = crear();

    document.dispatchEvent(tecla('F2'));

    expect(fixture.componentInstance.pulsado).toBe(1);
    expect(raiz.querySelector('button')?.getAttribute('aria-keyshortcuts')).toBe('F2');
  });

  it('enfoca el campo con su tecla suelta', () => {
    const { raiz } = crear();
    const evento = tecla('/');

    document.dispatchEvent(evento);

    expect(document.activeElement).toBe(raiz.querySelector('input'));
    expect(evento.defaultPrevented).toBe(true);
  });

  it('una tecla suelta no se roba mientras se escribe en otro campo', () => {
    const { raiz } = crear();
    const otro = raiz.querySelector<HTMLInputElement>('input.otro');
    otro?.focus();
    const evento = tecla('/');

    otro?.dispatchEvent(evento);

    expect(document.activeElement).toBe(otro);
    expect(evento.defaultPrevented).toBe(false);
  });

  it('una tecla de función sí vale aun escribiendo en un campo', () => {
    const { fixture, raiz } = crear();
    const otro = raiz.querySelector<HTMLInputElement>('input.otro');
    otro?.focus();

    otro?.dispatchEvent(tecla('F2'));

    expect(fixture.componentInstance.pulsado).toBe(1);
  });

  it('no hace nada con un modal abierto ni con el botón deshabilitado', () => {
    const { fixture } = crear();
    document.dispatchEvent(tecla('G'));
    expect(fixture.componentInstance.pulsado).toBe(0);

    const modal = document.createElement('div');
    modal.className = 'mat-mdc-dialog-container';
    document.body.append(modal);
    document.dispatchEvent(tecla('F2'));

    expect(fixture.componentInstance.pulsado).toBe(0);
  });
});
