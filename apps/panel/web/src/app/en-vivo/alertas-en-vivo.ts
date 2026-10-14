import { Injectable, inject } from '@angular/core';
import { Avisos } from '../compartido/avisos';
import { Nucleo } from '../nucleo/nucleo';
import { listarDispositivos, listarEquipos } from '../panel';
import { ApiUsuarios } from '../usuarios/api-usuarios';
import { DetectorDeAlertas, type AlertaParaMostrar, type EquipoConocido } from './alertas';
import { EnVivo } from './en-vivo';
import { Silencio } from './silencio';

/** Cada cuánto se mira si algún equipo dejó de avisar (no manda ningún aviso al callar). */
const REVISION_DE_SENAL_MS = 30_000;

/**
 * Los avisos en vivo del panel: usuario desactivado, equipo que se conecta, se va o pierde la señal,
 * equipo vinculado o retirado, nuevo ingreso. Se muestran con los mismos avisos de la página, estés
 * donde estés. Lo que esta misma página acaba de hacer (por ejemplo, desactivar a alguien) no se
 * avisa dos veces: ya se confirmó en el momento.
 */
@Injectable({ providedIn: 'root' })
export class AlertasEnVivo {
  private readonly avisos = inject(Avisos);
  private readonly enVivo = inject(EnVivo);
  private readonly api = inject(ApiUsuarios);
  private readonly silencio = inject(Silencio);
  private readonly detector = new DetectorDeAlertas(inject(Nucleo));
  private iniciado = false;

  /** Empieza a escuchar. Se llama una sola vez, al abrir el panel. */
  iniciar(): void {
    if (this.iniciado) {
      return;
    }
    this.iniciado = true;
    this.enVivo.iniciar();
    void this.conocer();
    this.enVivo.reconectado.subscribe(() => void this.conocer());
    this.enVivo.cambios.subscribe((aviso) => {
      const alertas = this.detector.alAvisar(aviso);
      if (!this.silencio.estaSilenciada(aviso.tabla)) {
        this.mostrar(alertas);
      }
    });
    setInterval(() => this.mostrar(this.detector.revisarSenal()), REVISION_DE_SENAL_MS);
  }

  private mostrar(alertas: readonly AlertaParaMostrar[]): void {
    for (const { tipo, titulo, mensaje } of alertas) {
      this.avisos[tipo](mensaje, titulo);
    }
  }

  /** Lo que ya existe, para que de ahí en adelante sólo avise lo que cambie. */
  private async conocer(): Promise<void> {
    try {
      const [usuarios, dispositivos, conectados] = await Promise.all([
        this.api.listar(),
        listarDispositivos(),
        listarEquipos(),
      ]);
      this.detector.conocerUsuarios(usuarios);
      const conexion = new Map(conectados.map((c) => [c.equipo, c]));
      const equipos: EquipoConocido[] = dispositivos.map((d) => {
        const c = conexion.get(d.id);
        return {
          id: d.id,
          nombre: d.nombre,
          unidad: d.unidad,
          enLinea: c !== undefined && c.estado !== 'fuera',
          vistoMs: c ? Date.parse(c.visto) || null : null,
          vinculado: d.vinculado !== null,
          retirado: d.retirado !== null,
        };
      });
      this.detector.conocerEquipos(equipos);
    } catch (error) {
      // Sin esto sólo se pierden los avisos de cambios: la página funciona igual.
      console.warn('No se pudo leer lo que ya existe para los avisos en vivo:', error);
    }
  }
}
