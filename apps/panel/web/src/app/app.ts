import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  signal,
} from '@angular/core';
import {
  ErrorPanel,
  listarDispositivos,
  listarEquipos,
  listarUnidades,
  registrarDispositivo,
  retirarDispositivo,
  type CodigoEmitido,
  type Dispositivo,
  type EquipoConectado,
  type TipoDispositivo,
  type Unidad,
} from './panel';

const CADA_CUANTO_MS = 15_000;

const TIPOS: { valor: TipoDispositivo; etiqueta: string }[] = [
  { valor: 'pc', etiqueta: 'PC' },
  { valor: 'celular', etiqueta: 'Celular' },
  { valor: 'visor', etiqueta: 'Visor (sólo lee)' },
];

const ETIQUETA_ESTADO: Record<string, string> = {
  esperando_codigo: 'Esperando su código',
  vinculado: 'Vinculado',
  retirado: 'Retirado',
  en_linea: 'En línea',
  caido: 'Caído',
  fuera: 'Fuera',
};

const MENSAJE_ERROR: Record<string, string> = {
  datos_invalidos: 'Revise el nombre, la unidad y el tipo del equipo.',
  nube: 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.',
  unidad_inexistente: 'Esa unidad operativa no existe. Elija una de la lista.',
};

/**
 * El panel de Limen: registrar y retirar equipos (con su código de vinculación) y ver
 * cuáles están conectados. No decide nada: el Worker aplica las reglas del dominio y
 * esta pantalla muestra lo que responde.
 */
@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App {
  protected readonly tipos = TIPOS;

  protected readonly dispositivos = signal<Dispositivo[]>([]);
  protected readonly equipos = signal<EquipoConectado[]>([]);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);

  protected readonly nombre = signal('');
  protected readonly unidades = signal<Unidad[]>([]);
  protected readonly unidad = signal('');
  protected readonly tipo = signal<TipoDispositivo>('pc');
  protected readonly enviando = signal(false);
  protected readonly errorFormulario = signal<string | null>(null);

  /** El código recién emitido: se muestra una sola vez, hasta que se cierre. */
  protected readonly emitido = signal<(CodigoEmitido & { vence: number }) | null>(null);
  private readonly ahora = signal(Date.now());
  protected readonly segundosRestantes = computed(() => {
    const codigo = this.emitido();
    return codigo ? Math.max(0, Math.floor((codigo.vence - this.ahora()) / 1000)) : 0;
  });

  constructor() {
    void this.cargar();
    const refresco = setInterval(() => {
      if (!document.hidden) {
        void this.cargar();
      }
    }, CADA_CUANTO_MS);
    const reloj = setInterval(() => this.ahora.set(Date.now()), 1000);
    inject(DestroyRef).onDestroy(() => {
      clearInterval(refresco);
      clearInterval(reloj);
    });
  }

  protected etiqueta(estado: string): string {
    return ETIQUETA_ESTADO[estado] ?? estado;
  }

  protected tiempo(segundos: number): string {
    const resto = String(segundos % 60).padStart(2, '0');
    return `${Math.floor(segundos / 60)}:${resto}`;
  }

  protected fecha(valor: string | null): string {
    return valor ? new Date(valor).toLocaleString('es-CR') : '';
  }

  protected escribir(evento: Event): void {
    this.nombre.set((evento.target as HTMLInputElement).value);
  }

  protected elegirUnidad(evento: Event): void {
    this.unidad.set((evento.target as HTMLSelectElement).value);
  }

  protected elegirTipo(evento: Event): void {
    this.tipo.set((evento.target as HTMLSelectElement).value as TipoDispositivo);
  }

  protected async registrar(evento: Event): Promise<void> {
    evento.preventDefault();
    if (this.enviando()) {
      return;
    }
    this.enviando.set(true);
    this.errorFormulario.set(null);
    try {
      const codigo = await registrarDispositivo({
        nombre: this.nombre(),
        unidad: this.unidad(),
        tipo: this.tipo(),
      });
      this.emitido.set({ ...codigo, vence: Date.now() + codigo.vigencia_minutos * 60_000 });
      this.nombre.set('');
      await this.cargar();
    } catch (error) {
      this.errorFormulario.set(this.mensaje(error));
    } finally {
      this.enviando.set(false);
    }
  }

  protected async retirar(dispositivo: Dispositivo): Promise<void> {
    if (!confirm(`¿Retirar «${dispositivo.nombre}»? El equipo pierde el acceso al instante.`)) {
      return;
    }
    try {
      await retirarDispositivo(dispositivo.id);
      await this.cargar();
    } catch (error) {
      this.error.set(this.mensaje(error));
    }
  }

  protected cerrarCodigo(): void {
    this.emitido.set(null);
  }

  private async cargar(): Promise<void> {
    try {
      const [dispositivos, equipos, unidades] = await Promise.all([
        listarDispositivos(),
        listarEquipos(),
        listarUnidades(),
      ]);
      this.dispositivos.set(dispositivos);
      this.equipos.set(equipos);
      this.unidades.set(unidades);
      // Con una sola unidad no hay nada que decidir: queda elegida.
      if (this.unidad() === '' && unidades.length === 1) {
        this.unidad.set(unidades[0]?.codigo ?? '');
      }
      this.error.set(null);
    } catch (error) {
      this.error.set(this.mensaje(error));
    } finally {
      this.cargando.set(false);
    }
  }

  private mensaje(error: unknown): string {
    const codigo = error instanceof ErrorPanel ? error.codigo : '';
    return MENSAJE_ERROR[codigo] ?? 'No se pudo completar la operación. Intente de nuevo.';
  }
}
