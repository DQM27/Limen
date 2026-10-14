/**
 * Un cliente mínimo de SurrealDB por WebSocket (JSON-RPC): entrar con un token, elegir la base y
 * suscribirse a los cambios de una tabla (`live`). Sólo lo que el panel necesita, sin la librería
 * completa. No decide nada: entrega los avisos tal como llegan.
 *
 * El `WebSocket` se recibe por parámetro para poder probarlo sin red.
 */

/** Lo que avisa la base: qué pasó con qué registro y cómo quedó. */
export interface AvisoDeTabla {
  tabla: string;
  /** `CREATE`, `UPDATE` o `DELETE`. */
  accion: string;
  /** El registro como lo manda la base, por ejemplo ``conexion:`abc` `` o `usuario:u'…'`. */
  registro: string;
  /** Cómo quedó el registro (en un `DELETE`, cómo estaba). */
  datos: Record<string, unknown>;
}

export interface SocketMinimo {
  onopen: ((evento: unknown) => void) | null;
  onmessage: ((evento: { data: unknown }) => void) | null;
  onclose: ((evento: unknown) => void) | null;
  onerror: ((evento: unknown) => void) | null;
  send(datos: string): void;
  close(): void;
}

export type FabricaDeSocket = (url: string) => SocketMinimo;

interface Respuesta {
  id?: number;
  result?: unknown;
  error?: { message?: string };
}

interface Notificacion {
  action: string;
  id: string;
  record?: unknown;
  result?: Record<string, unknown>;
}

const SOCKET_REAL: FabricaDeSocket = (url) => new WebSocket(url) as unknown as SocketMinimo;

export class ErrorDeSurreal extends Error {}

export class ClienteSurreal {
  private socket: SocketMinimo | null = null;
  private siguiente = 0;
  private readonly esperando = new Map<
    number,
    { resolver: (valor: unknown) => void; rechazar: (error: Error) => void }
  >();
  /** Qué tabla corresponde a cada suscripción en vivo. */
  private readonly suscripciones = new Map<string, string>();
  private cerradoAdrede = false;

  constructor(
    private readonly alAvisar: (aviso: AvisoDeTabla) => void,
    private readonly alCerrarse: () => void,
    private readonly fabrica: FabricaDeSocket = SOCKET_REAL,
  ) {}

  /** Abre la conexión, entra con el token y elige la base. */
  async conectar(url: string, token: string): Promise<void> {
    this.cerradoAdrede = false;
    const socket = this.fabrica(`${url.replace(/\/$/, '')}/rpc`);
    this.socket = socket;
    await new Promise<void>((resolver, rechazar) => {
      socket.onopen = () => resolver();
      socket.onerror = () => rechazar(new ErrorDeSurreal('no se pudo conectar'));
    });
    socket.onerror = null;
    socket.onmessage = (evento) => this.recibir(String(evento.data));
    socket.onclose = () => this.alCerrar();
    await this.pedir('authenticate', [token]);
    await this.pedir('use', ['limen', 'principal']);
  }

  /** Avisa de cada cambio de `tabla` desde ahora. */
  async suscribir(tabla: string): Promise<void> {
    const id = (await this.pedir('live', [tabla])) as string;
    this.suscripciones.set(id, tabla);
  }

  cerrar(): void {
    this.cerradoAdrede = true;
    this.socket?.close();
  }

  private pedir(method: string, params: unknown[]): Promise<unknown> {
    return new Promise((resolver, rechazar) => {
      const socket = this.socket;
      if (!socket) {
        rechazar(new ErrorDeSurreal('sin conexión'));
        return;
      }
      const id = ++this.siguiente;
      this.esperando.set(id, { resolver, rechazar });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }

  private recibir(texto: string): void {
    let mensaje: Respuesta;
    try {
      mensaje = JSON.parse(texto) as Respuesta;
    } catch {
      return;
    }
    if (mensaje.id !== undefined && this.esperando.has(mensaje.id)) {
      const pendiente = this.esperando.get(mensaje.id);
      this.esperando.delete(mensaje.id);
      if (mensaje.error) {
        pendiente?.rechazar(new ErrorDeSurreal(mensaje.error.message ?? 'error de la base'));
      } else {
        pendiente?.resolver(mensaje.result);
      }
      return;
    }
    // Sin `id` de petición: es un aviso de una suscripción en vivo.
    const aviso = mensaje.result as Notificacion | undefined;
    const tabla = aviso ? this.suscripciones.get(aviso.id) : undefined;
    if (aviso && tabla) {
      this.alAvisar({
        tabla,
        accion: aviso.action,
        registro: String(aviso.record ?? aviso.result?.['id'] ?? ''),
        datos: aviso.result ?? {},
      });
    }
  }

  private alCerrar(): void {
    for (const { rechazar } of this.esperando.values()) {
      rechazar(new ErrorDeSurreal('la conexión se cerró'));
    }
    this.esperando.clear();
    this.suscripciones.clear();
    this.socket = null;
    if (!this.cerradoAdrede) {
      this.alCerrarse();
    }
  }
}
