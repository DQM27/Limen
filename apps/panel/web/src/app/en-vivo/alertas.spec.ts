import { beforeEach, describe, expect, it } from 'vitest';
import { Nucleo, type EstadoDeConexion } from '../nucleo/nucleo';
import type { AvisoDeTabla } from './cliente-surreal';
import { DetectorDeAlertas, idDeRegistro, type EquipoConocido } from './alertas';

/** El núcleo de mentira decide como el de verdad: más de diez minutos sin avisar es «sin señal». */
class NucleoFalso extends Nucleo {
  filasQueCoinciden(): boolean[] {
    return [];
  }
  resaltar(): never[] {
    return [];
  }
  nombreAlEscribir(texto: string): string {
    return texto;
  }
  datosDeUsuario(): never {
    throw new Error('no se usa');
  }
  mensajeDeError(): string {
    return '';
  }
  empresaAlEscribir(texto: string): string {
    return texto;
  }
  praindVencido(): boolean {
    return false;
  }
  estadoDeConexion(enLinea: boolean, vistoMs: number, ahoraMs: number): EstadoDeConexion {
    if (!enLinea) {
      return 'fuera';
    }
    return ahoraMs - vistoMs > 10 * 60_000 ? 'caido' : 'en_linea';
  }
}

const A1 = '0199c7a2-0000-7000-8000-000000000001';
const ANA = '0199c7a2-0000-7000-8000-0000000000aa';
const AHORA = Date.UTC(2026, 9, 14, 17, 0, 0);
const hace = (minutos: number): string => new Date(AHORA - minutos * 60_000).toISOString();

let reloj = AHORA;
let detector: DetectorDeAlertas;

const equipo = (cambios: Partial<EquipoConocido> = {}): EquipoConocido => ({
  id: A1,
  nombre: 'A2',
  unidad: 'BRISAS',
  enLinea: true,
  vistoMs: AHORA - 60_000,
  vinculado: true,
  retirado: false,
  ...cambios,
});

const conexion = (enLinea: boolean, visto = hace(0)): AvisoDeTabla => ({
  tabla: 'conexion',
  accion: 'UPDATE',
  registro: `conexion:\`${A1}\``,
  datos: { equipo: A1, unidad: 'BRISAS', en_linea: enLinea, visto },
});

const usuario = (activo: boolean, nombre = 'ANA MORA', accion = 'UPDATE'): AvisoDeTabla => ({
  tabla: 'usuario',
  accion,
  registro: `usuario:u'${ANA}'`,
  datos: { id: `usuario:u'${ANA}'`, nombre, activo },
});

beforeEach(() => {
  reloj = AHORA;
  detector = new DetectorDeAlertas(new NucleoFalso(), () => reloj);
  detector.conocerUsuarios([{ id: ANA, nombre: 'ANA MORA', activo: true }]);
  detector.conocerEquipos([equipo()]);
});

describe('usuarios', () => {
  it('avisa cuando se desactiva un usuario', () => {
    expect(detector.alAvisar(usuario(false))).toEqual([
      {
        tipo: 'error',
        titulo: 'Usuario desactivado',
        mensaje: 'ANA MORA ya no puede iniciar sesión.',
      },
    ]);
  });

  it('avisa cuando se vuelve a activar', () => {
    detector.alAvisar(usuario(false));
    expect(detector.alAvisar(usuario(true))).toEqual([
      {
        tipo: 'exito',
        titulo: 'Usuario activado',
        mensaje: 'ANA MORA puede iniciar sesión otra vez.',
      },
    ]);
  });

  it('un cambio que no toca «activo» (el nombre, la clave) no avisa', () => {
    expect(detector.alAvisar(usuario(true, 'ANA MORA VEGA'))).toEqual([]);
  });

  it('un usuario que no conocía se avisa como nuevo', () => {
    detector.conocerUsuarios([]);
    expect(detector.alAvisar(usuario(true, 'LUIS SOTO', 'CREATE'))).toMatchObject([
      { tipo: 'info', titulo: 'Usuario nuevo' },
    ]);
  });
});

describe('equipos', () => {
  it('un latido de un equipo que sigue en línea no avisa', () => {
    expect(detector.alAvisar(conexion(true))).toEqual([]);
  });

  it('avisa cuando un equipo se desconecta de forma normal', () => {
    expect(detector.alAvisar(conexion(false))).toEqual([
      { tipo: 'info', titulo: 'Equipo desconectado', mensaje: 'A2 (BRISAS) cerró la app.' },
    ]);
  });

  it('avisa cuando un equipo vuelve a conectarse', () => {
    detector.alAvisar(conexion(false));
    expect(detector.alAvisar(conexion(true))).toEqual([
      { tipo: 'info', titulo: 'Equipo conectado', mensaje: 'A2 (BRISAS) está en línea.' },
    ]);
  });

  it('un equipo que deja de avisar se da por sin señal cuando pasa el tiempo, sin ningún aviso de la base', () => {
    expect(detector.revisarSenal()).toEqual([]);
    reloj = AHORA + 12 * 60_000;
    expect(detector.revisarSenal()).toEqual([
      {
        tipo: 'error',
        titulo: 'Equipo sin señal',
        mensaje: 'A2 (BRISAS) dejó de avisar que está abierto.',
      },
    ]);
    expect(detector.revisarSenal(), 'no se repite').toEqual([]);
  });

  it('si estaba sin señal y luego se despide, no es una novedad; si vuelve, sí', () => {
    reloj = AHORA + 12 * 60_000;
    detector.revisarSenal();
    expect(detector.alAvisar(conexion(false, hace(-12)))).toEqual([]);
    expect(detector.alAvisar(conexion(true, hace(-12)))).toMatchObject([
      { titulo: 'Equipo conectado' },
    ]);
  });

  it('usa el nombre del equipo si lo conoce y, si no, el comienzo de su ID', () => {
    detector.conocerEquipos([]);
    expect(detector.alAvisar(conexion(false))).toMatchObject([
      { mensaje: `${A1.slice(0, 8)} (BRISAS) cerró la app.` },
    ]);
  });
});

describe('dispositivos', () => {
  const dispositivo = (extra: Record<string, unknown>): AvisoDeTabla => ({
    tabla: 'dispositivo',
    accion: 'UPDATE',
    registro: `dispositivo:\`${A1}\``,
    datos: { id: `dispositivo:\`${A1}\``, nombre: 'A2', unidad: 'BRISAS', ...extra },
  });

  it('avisa cuando un equipo se vincula y cuando se retira', () => {
    detector.conocerEquipos([equipo({ vinculado: false })]);
    expect(detector.alAvisar(dispositivo({ vinculado: '2026-10-14T16:00:00Z' }))).toMatchObject([
      { tipo: 'exito', titulo: 'Equipo vinculado' },
    ]);
    expect(
      detector.alAvisar(
        dispositivo({ vinculado: '2026-10-14T16:00:00Z', retirado: '2026-10-14T16:30:00Z' }),
      ),
    ).toMatchObject([{ tipo: 'error', titulo: 'Equipo retirado' }]);
  });

  it('avisa cuando se registra un equipo nuevo', () => {
    detector.conocerEquipos([]);
    expect(detector.alAvisar({ ...dispositivo({}), accion: 'CREATE' })).toMatchObject([
      { titulo: 'Equipo registrado' },
    ]);
  });
});

describe('ingresos', () => {
  const hecho = (tipo: string, via = 'CONTRATISTA'): AvisoDeTabla => ({
    tabla: 'hecho',
    accion: 'CREATE',
    registro: 'hecho:abc',
    datos: { tipo, via, unidad: 'CARTAGO', en: '2026-10-14T20:05:09Z' },
  });

  it('avisa de cada entrada y salida con su vía y su unidad', () => {
    const [entrada] = detector.alAvisar(hecho('ENTRADA'));
    expect(entrada).toMatchObject({ tipo: 'info', titulo: 'Nuevo ingreso' });
    expect(entrada?.mensaje).toMatch(/^Contratista · CARTAGO · \d{2}:\d{2}:\d{2}$/);
    expect(detector.alAvisar(hecho('SALIDA', 'KOF'))).toMatchObject([
      { titulo: 'Salida registrada', mensaje: expect.stringContaining('Personal KOF · CARTAGO') },
    ]);
  });

  it('lo que no es un hecho nuevo, o una tabla que no se sigue, no avisa', () => {
    expect(detector.alAvisar({ ...hecho('ENTRADA'), accion: 'UPDATE' })).toEqual([]);
    expect(
      detector.alAvisar({ tabla: 'otra', accion: 'CREATE', registro: 'x', datos: {} }),
    ).toEqual([]);
  });
});

describe('idDeRegistro', () => {
  it('saca el UUID de las dos formas en que la base escribe un registro', () => {
    expect(idDeRegistro(`usuario:u'${ANA}'`)).toBe(ANA);
    expect(idDeRegistro(`dispositivo:\`${A1}\``)).toBe(A1);
    expect(idDeRegistro('conexion:`a1`')).toBe('conexion:`a1`');
  });
});
