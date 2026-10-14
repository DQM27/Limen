import { describe, expect, it, vi } from 'vitest';
import { ClienteSurreal, type AvisoDeTabla, type SocketMinimo } from './cliente-surreal';

/** Un WebSocket de mentira que hace de servidor de SurrealDB: contesta lo que se le pide. */
class SocketFalso implements SocketMinimo {
  onopen: ((evento: unknown) => void) | null = null;
  onmessage: ((evento: { data: unknown }) => void) | null = null;
  onclose: ((evento: unknown) => void) | null = null;
  onerror: ((evento: unknown) => void) | null = null;
  readonly enviados: { id: number; method: string; params: unknown[] }[] = [];
  cerrado = false;

  constructor(
    readonly url: string,
    private readonly rechazarToken = false,
  ) {
    queueMicrotask(() => this.onopen?.({}));
  }

  send(datos: string): void {
    const pedido = JSON.parse(datos) as { id: number; method: string; params: unknown[] };
    this.enviados.push(pedido);
    queueMicrotask(() => {
      if (pedido.method === 'authenticate' && this.rechazarToken) {
        this.responder({ id: pedido.id, error: { message: 'token inválido' } });
      } else if (pedido.method === 'live') {
        this.responder({ id: pedido.id, result: `vivo-${String(pedido.params[0])}` });
      } else {
        this.responder({ id: pedido.id, result: null });
      }
    });
  }

  responder(mensaje: unknown): void {
    this.onmessage?.({ data: JSON.stringify(mensaje) });
  }

  close(): void {
    this.cerrado = true;
    this.onclose?.({});
  }
}

function cliente(rechazarToken = false) {
  const avisos: AvisoDeTabla[] = [];
  const alCerrarse = vi.fn();
  let socket!: SocketFalso;
  const clienteSurreal = new ClienteSurreal(
    (aviso) => avisos.push(aviso),
    alCerrarse,
    (url) => (socket = new SocketFalso(url, rechazarToken)),
  );
  return { clienteSurreal, avisos, alCerrarse, socket: () => socket };
}

describe('ClienteSurreal', () => {
  it('se conecta a /rpc, entra con el token y elige la base', async () => {
    const { clienteSurreal, socket } = cliente();
    await clienteSurreal.conectar('wss://limen.surreal.cloud/', 'mi-token');
    expect(socket().url).toBe('wss://limen.surreal.cloud/rpc');
    expect(socket().enviados.map((p) => [p.method, p.params])).toEqual([
      ['authenticate', ['mi-token']],
      ['use', ['limen', 'principal']],
    ]);
  });

  it('un token rechazado por la base es un error al conectar', async () => {
    const { clienteSurreal } = cliente(true);
    await expect(clienteSurreal.conectar('wss://x', 'malo')).rejects.toThrow('token inválido');
  });

  it('avisa de cada cambio con su tabla, su acción y cómo quedó el registro', async () => {
    const { clienteSurreal, avisos, socket } = cliente();
    await clienteSurreal.conectar('wss://x', 't');
    await clienteSurreal.suscribir('conexion');
    await clienteSurreal.suscribir('usuario');

    socket().responder({
      result: {
        action: 'UPDATE',
        id: 'vivo-conexion',
        record: 'conexion:`a1`',
        result: { en_linea: true, equipo: 'a1' },
      },
    });
    socket().responder({
      result: {
        action: 'CREATE',
        id: 'vivo-usuario',
        result: { id: "usuario:u'abc'", activo: true },
      },
    });

    expect(avisos).toEqual([
      {
        tabla: 'conexion',
        accion: 'UPDATE',
        registro: 'conexion:`a1`',
        datos: { en_linea: true, equipo: 'a1' },
      },
      {
        tabla: 'usuario',
        accion: 'CREATE',
        registro: "usuario:u'abc'",
        datos: { id: "usuario:u'abc'", activo: true },
      },
    ]);
  });

  it('un aviso de una suscripción que no es nuestra, o un texto ilegible, se ignora', async () => {
    const { clienteSurreal, avisos, socket } = cliente();
    await clienteSurreal.conectar('wss://x', 't');
    await clienteSurreal.suscribir('conexion');
    socket().responder({ result: { action: 'CREATE', id: 'otra-cosa', result: {} } });
    socket().onmessage?.({ data: 'no es json' });
    expect(avisos).toEqual([]);
  });

  it('si la conexión se corta sin que se pida, lo avisa; si se cierra a propósito, no', async () => {
    const corte = cliente();
    await corte.clienteSurreal.conectar('wss://x', 't');
    corte.socket().onclose?.({});
    expect(corte.alCerrarse).toHaveBeenCalledTimes(1);

    const adrede = cliente();
    await adrede.clienteSurreal.conectar('wss://x', 't');
    adrede.clienteSurreal.cerrar();
    expect(adrede.socket().cerrado).toBe(true);
    expect(adrede.alCerrarse).not.toHaveBeenCalled();
  });
});
