import { Injectable, inject, signal } from '@angular/core';
import { Subject } from 'rxjs';
import { ClienteSurreal, type AvisoDeTabla } from './cliente-surreal';

/** Cómo va la conexión en vivo con la base. */
export type EstadoEnVivo =
  /** Todavía no se intentó o se está conectando. */
  | 'conectando'
  /** Conectado: los cambios llegan al instante. */
  | 'en_vivo'
  /** Se cortó: se reintenta solo; mientras tanto la página se refresca cada tanto. */
  | 'sin_conexion'
  /** El Worker no tiene configurado el tiempo real: la página funciona igual, sin él. */
  | 'no_disponible';

/** Las tablas que la página sigue en vivo. */
export const TABLAS_EN_VIVO = ['conexion', 'usuario', 'dispositivo', 'hecho'] as const;

/** Lo que el Worker da para conectarse: un token de sólo lectura de una hora. */
export interface AccesoEnVivo {
  token: string;
  url: string;
  /** Cuándo vence el token, en milisegundos. */
  vence: number;
}

/** Cuánto antes de que venza el token se renueva (se vuelve a conectar con uno nuevo). */
const MARGEN_DE_RENOVACION_MS = 5 * 60_000;
const ESPERA_MINIMA_MS = 1_000;
const ESPERA_MAXIMA_MS = 30_000;

/** Pide al Worker el acceso para conectarse en vivo. `null` si no está configurado. */
export async function pedirAccesoEnVivo(): Promise<AccesoEnVivo | null> {
  const respuesta = await fetch('/api/en-vivo', { method: 'POST' });
  if (respuesta.status === 503) {
    return null;
  }
  if (!respuesta.ok) {
    throw new Error(`en vivo: ${respuesta.status}`);
  }
  return (await respuesta.json()) as AccesoEnVivo;
}

/**
 * La conexión en vivo con la base, para todo el panel: se conecta una sola vez, se suscribe a las
 * tablas que importan y reparte lo que llega. Si se corta, reintenta con espera creciente; antes
 * de que venza el token pide otro y se vuelve a conectar. Quien necesite saber de un cambio se
 * suscribe a {@link EnVivo.cambios}; al reconectar emite `reconectado`, porque pudo perderse algo.
 */
@Injectable({ providedIn: 'root' })
export class EnVivo {
  readonly estado = signal<EstadoEnVivo>('conectando');
  /** Cada cambio que avisa la base. */
  readonly cambios = new Subject<AvisoDeTabla>();
  /** Se volvió a conectar después de un corte: conviene volver a leer todo. */
  readonly reconectado = new Subject<void>();

  private cliente: ClienteSurreal | null = null;
  private intentos = 0;
  private temporizador: ReturnType<typeof setTimeout> | null = null;
  private iniciado = false;
  private yaConectoUnaVez = false;

  /** Empieza a conectarse. Se llama una sola vez, al abrir el panel. */
  iniciar(): void {
    if (this.iniciado) {
      return;
    }
    this.iniciado = true;
    void this.conectar();
  }

  /** Para quien lo necesite en pruebas: reemplaza cómo se piden el acceso y el cliente. */
  protected pedirAcceso(): Promise<AccesoEnVivo | null> {
    return pedirAccesoEnVivo();
  }

  protected crearCliente(): ClienteSurreal {
    return new ClienteSurreal(
      (aviso) => this.cambios.next(aviso),
      () => this.alCortarse(),
    );
  }

  /** `renovando`: se cambia de token a propósito, sin que el estado parpadee ni parezca un corte. */
  private async conectar(renovando = false): Promise<void> {
    if (!renovando) {
      this.estado.set(this.yaConectoUnaVez ? 'sin_conexion' : 'conectando');
    }
    try {
      const acceso = await this.pedirAcceso();
      if (acceso === null) {
        this.estado.set('no_disponible');
        return;
      }
      const cliente = this.crearCliente();
      await cliente.conectar(acceso.url, acceso.token);
      for (const tabla of TABLAS_EN_VIVO) {
        await cliente.suscribir(tabla);
      }
      this.cliente = cliente;
      this.intentos = 0;
      this.estado.set('en_vivo');
      if (this.yaConectoUnaVez && !renovando) {
        this.reconectado.next();
      }
      this.yaConectoUnaVez = true;
      this.programarRenovacion(acceso.vence);
    } catch (error) {
      console.warn('No se pudo conectar en vivo:', error);
      this.reintentar();
    }
  }

  private alCortarse(): void {
    this.cliente = null;
    this.estado.set('sin_conexion');
    this.reintentar();
  }

  private reintentar(): void {
    this.estado.set(this.yaConectoUnaVez ? 'sin_conexion' : 'conectando');
    const espera = Math.min(ESPERA_MINIMA_MS * 2 ** this.intentos, ESPERA_MAXIMA_MS);
    this.intentos += 1;
    this.programar(() => void this.conectar(), espera);
  }

  /** Antes de que venza el token, se conecta de nuevo con uno nuevo. */
  private programarRenovacion(vence: number): void {
    const falta = Math.max(vence - Date.now() - MARGEN_DE_RENOVACION_MS, ESPERA_MINIMA_MS);
    this.programar(() => {
      this.cliente?.cerrar();
      this.cliente = null;
      void this.conectar(true);
    }, falta);
  }

  private programar(accion: () => void, ms: number): void {
    if (this.temporizador !== null) {
      clearTimeout(this.temporizador);
    }
    this.temporizador = setTimeout(accion, ms);
  }
}
