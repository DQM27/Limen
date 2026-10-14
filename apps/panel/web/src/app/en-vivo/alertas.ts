import { hora24 } from '../compartido/fecha';
import type { EstadoDeConexion, Nucleo } from '../nucleo/nucleo';
import type { AvisoDeTabla } from './cliente-surreal';

/** Lo que se le muestra a quien tiene el panel abierto, con la forma de los avisos de la página. */
export interface AlertaParaMostrar {
  tipo: 'exito' | 'error' | 'info';
  titulo: string;
  mensaje: string;
}

/** Un usuario como lo conoce el panel, para notar cuando cambia. */
export interface UsuarioConocido {
  id: string;
  nombre: string;
  activo: boolean;
}

/** Un equipo como lo conoce el panel: lo que se registró y cómo está conectado. */
export interface EquipoConocido {
  id: string;
  nombre: string;
  unidad: string;
  /** Lo último que dijo de sí mismo (`true` al abrir y en cada latido, `false` al despedirse). */
  enLinea: boolean;
  /** Su última señal, en milisegundos; `null` si nunca avisó. */
  vistoMs: number | null;
  vinculado: boolean;
  retirado: boolean;
}

const VIA: Record<string, string> = {
  CONTRATISTA: 'Contratista',
  PROVEEDOR: 'Proveedor',
  CORREO: 'Correo',
  KOF: 'Personal KOF',
};

/** El UUID dentro del texto de un registro: `usuario:u'0199…'` o `` dispositivo:`0199…` ``. */
export function idDeRegistro(registro: unknown): string {
  const texto = String(registro ?? '');
  return /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i.exec(texto)?.[0] ?? texto;
}

const texto = (valor: unknown): string => (typeof valor === 'string' ? valor : '');

/**
 * Decide qué cambio merece un aviso. No tiene reglas de negocio propias: cómo está conectado un
 * equipo lo pregunta al núcleo, y de lo demás sólo compara cómo estaba con cómo quedó. Lo que no
 * conoce de antes (la primera vez que lo ve) lo aprende sin avisar.
 */
export class DetectorDeAlertas {
  private readonly usuarios = new Map<string, UsuarioConocido>();
  private readonly equipos = new Map<string, EquipoConocido>();
  private readonly estados = new Map<string, EstadoDeConexion>();

  constructor(
    private readonly nucleo: Nucleo,
    private readonly ahora: () => number = Date.now,
  ) {}

  /** Lo que ya existe cuando se abre el panel: de ahí en adelante, sólo lo que cambie avisa. */
  conocerUsuarios(usuarios: readonly UsuarioConocido[]): void {
    this.usuarios.clear();
    for (const usuario of usuarios) {
      this.usuarios.set(usuario.id, usuario);
    }
  }

  conocerEquipos(equipos: readonly EquipoConocido[]): void {
    this.equipos.clear();
    this.estados.clear();
    for (const equipo of equipos) {
      this.equipos.set(equipo.id, equipo);
      this.estados.set(equipo.id, this.estadoDe(equipo));
    }
  }

  /** Reacciona a un aviso de la base. */
  alAvisar(aviso: AvisoDeTabla): AlertaParaMostrar[] {
    switch (aviso.tabla) {
      case 'usuario':
        return this.deUsuario(aviso);
      case 'conexion':
        return this.deConexion(aviso);
      case 'dispositivo':
        return this.deDispositivo(aviso);
      case 'hecho':
        return this.deHecho(aviso);
      default:
        return [];
    }
  }

  /**
   * Un equipo que deja de avisar no manda ningún aviso: hay que mirar la hora. Se llama cada tanto
   * y avisa de los que pasaron a «sin señal».
   */
  revisarSenal(): AlertaParaMostrar[] {
    const alertas: AlertaParaMostrar[] = [];
    for (const equipo of this.equipos.values()) {
      const nuevo = this.estadoDe(equipo);
      const anterior = this.estados.get(equipo.id);
      this.estados.set(equipo.id, nuevo);
      if (nuevo === 'caido' && anterior === 'en_linea') {
        alertas.push(this.sinSenal(equipo));
      }
    }
    return alertas;
  }

  private estadoDe(equipo: EquipoConocido): EstadoDeConexion {
    return this.nucleo.estadoDeConexion(equipo.enLinea, equipo.vistoMs ?? 0, this.ahora());
  }

  private nombreDe(id: string): string {
    return this.equipos.get(id)?.nombre ?? id.slice(0, 8);
  }

  private sinSenal(equipo: EquipoConocido): AlertaParaMostrar {
    return {
      tipo: 'error',
      titulo: 'Equipo sin señal',
      mensaje: `${equipo.nombre} (${equipo.unidad}) dejó de avisar que está abierto.`,
    };
  }

  private deUsuario(aviso: AvisoDeTabla): AlertaParaMostrar[] {
    if (aviso.accion === 'DELETE') {
      return [];
    }
    const id = idDeRegistro(aviso.datos['id'] ?? aviso.registro);
    const nombre = texto(aviso.datos['nombre']);
    const activo = aviso.datos['activo'] === true;
    const anterior = this.usuarios.get(id);
    this.usuarios.set(id, { id, nombre, activo });

    if (!anterior) {
      return [
        { tipo: 'info', titulo: 'Usuario nuevo', mensaje: `${nombre} se agregó a los usuarios.` },
      ];
    }
    if (anterior.activo && !activo) {
      return [
        {
          tipo: 'error',
          titulo: 'Usuario desactivado',
          mensaje: `${nombre} ya no puede iniciar sesión.`,
        },
      ];
    }
    if (!anterior.activo && activo) {
      return [
        {
          tipo: 'exito',
          titulo: 'Usuario activado',
          mensaje: `${nombre} puede iniciar sesión otra vez.`,
        },
      ];
    }
    return [];
  }

  private deConexion(aviso: AvisoDeTabla): AlertaParaMostrar[] {
    if (aviso.accion === 'DELETE') {
      return [];
    }
    const id = texto(aviso.datos['equipo']) || idDeRegistro(aviso.registro);
    const conocido = this.equipos.get(id);
    const equipo: EquipoConocido = {
      id,
      nombre: conocido?.nombre ?? id.slice(0, 8),
      unidad: texto(aviso.datos['unidad']) || (conocido?.unidad ?? ''),
      enLinea: aviso.datos['en_linea'] === true,
      vistoMs: Date.parse(texto(aviso.datos['visto'])) || null,
      vinculado: conocido?.vinculado ?? true,
      retirado: conocido?.retirado ?? false,
    };
    const anterior = this.estados.get(id);
    const nuevo = this.estadoDe(equipo);
    this.equipos.set(id, equipo);
    this.estados.set(id, nuevo);

    if (nuevo === anterior) {
      return []; // un latido: sigue igual
    }
    if (nuevo === 'en_linea') {
      return [
        {
          tipo: 'info',
          titulo: 'Equipo conectado',
          mensaje: `${equipo.nombre} (${equipo.unidad}) está en línea.`,
        },
      ];
    }
    if (nuevo === 'fuera') {
      // Si ya estaba sin señal, no es una novedad: sólo confirma que se cerró.
      return anterior === 'caido'
        ? []
        : [
            {
              tipo: 'info',
              titulo: 'Equipo desconectado',
              mensaje: `${equipo.nombre} (${equipo.unidad}) cerró la app.`,
            },
          ];
    }
    return [this.sinSenal(equipo)];
  }

  private deDispositivo(aviso: AvisoDeTabla): AlertaParaMostrar[] {
    if (aviso.accion === 'DELETE') {
      return [];
    }
    const id = idDeRegistro(aviso.datos['id'] ?? aviso.registro);
    const anterior = this.equipos.get(id);
    const equipo: EquipoConocido = {
      id,
      nombre: texto(aviso.datos['nombre']),
      unidad: texto(aviso.datos['unidad']),
      enLinea: anterior?.enLinea ?? false,
      vistoMs: anterior?.vistoMs ?? null,
      vinculado: Boolean(aviso.datos['vinculado']),
      retirado: Boolean(aviso.datos['retirado']),
    };
    this.equipos.set(id, equipo);
    if (!this.estados.has(id)) {
      this.estados.set(id, 'fuera');
    }

    if (!anterior) {
      return [
        {
          tipo: 'info',
          titulo: 'Equipo registrado',
          mensaje: `${equipo.nombre} (${equipo.unidad}) espera su código de vinculación.`,
        },
      ];
    }
    if (!anterior.retirado && equipo.retirado) {
      return [
        {
          tipo: 'error',
          titulo: 'Equipo retirado',
          mensaje: `${equipo.nombre} (${equipo.unidad}) ya no tiene acceso a la nube.`,
        },
      ];
    }
    if (!anterior.vinculado && equipo.vinculado) {
      return [
        {
          tipo: 'exito',
          titulo: 'Equipo vinculado',
          mensaje: `${equipo.nombre} (${equipo.unidad}) ya se conectó con su código.`,
        },
      ];
    }
    return [];
  }

  private deHecho(aviso: AvisoDeTabla): AlertaParaMostrar[] {
    if (aviso.accion !== 'CREATE') {
      return [];
    }
    const via = VIA[texto(aviso.datos['via'])] ?? texto(aviso.datos['via']);
    const donde = `${via} · ${texto(aviso.datos['unidad'])} · ${hora24(texto(aviso.datos['en']))}`;
    return [
      aviso.datos['tipo'] === 'ENTRADA'
        ? { tipo: 'info', titulo: 'Nuevo ingreso', mensaje: donde }
        : { tipo: 'info', titulo: 'Salida registrada', mensaje: donde },
    ];
  }
}
