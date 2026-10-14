import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  signal,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import type { ColDef } from 'ag-grid-community';
import { debounceTime, filter, merge } from 'rxjs';
import { Avisos } from '../compartido/avisos';
import { fechaHora } from '../compartido/fecha';
import { celdaResaltada } from '../compartido/resaltado';
import { Tabla } from '../compartido/tabla';
import { EnVivo } from '../en-vivo/en-vivo';
import { Nucleo, type CampoDeTexto } from '../nucleo/nucleo';
import {
  ErrorPanel,
  type AtajoDeFecha,
  type Ingreso,
  type Unidad,
  type ViaDeIngreso,
} from '../panel';
import { ApiIngresos } from './api-ingresos';
import { VIAS, etiquetaDeVia, textoGafete, textoMedio, textoOperador } from './etiquetas';

const CADA_CUANTO_MS = 15_000;
const PERSONALIZADO = 'PERSONALIZADO';

type Estado = 'adentro' | 'historial';

/**
 * Los ingresos de todas las unidades, por las cuatro vías: **lo que está pasando** (quién está
 * adentro ahora) y **lo que ya pasó** (el historial), con botones para pasar de uno a otro y para
 * acotar por vía y por unidad. La pantalla no decide nada: los accesos rápidos de fecha, el
 * rango, el orden y el buscador son del núcleo. Se actualiza sola con cada entrada y salida.
 */
@Component({
  selector: 'app-ingresos-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatButtonToggleModule, MatIconModule, MatMenuModule, Tabla],
  templateUrl: './ingresos-pagina.html',
  styleUrl: './ingresos-pagina.scss',
})
export class IngresosPagina {
  private readonly nucleo = inject(Nucleo);
  private readonly api = inject(ApiIngresos);
  private readonly enVivo = inject(EnVivo);
  private readonly avisos = inject(Avisos);

  protected readonly vias = VIAS;

  protected readonly estado = signal<Estado>('adentro');
  protected readonly via = signal<ViaDeIngreso | ''>('');
  protected readonly unidad = signal('');
  protected readonly unidades = signal<Unidad[]>([]);
  protected readonly atajos = signal<AtajoDeFecha[]>([]);
  /** El código del acceso rápido elegido, o `PERSONALIZADO` si el rango se escribió a mano. */
  protected readonly atajo = signal('');
  protected readonly desde = signal('');
  protected readonly hasta = signal('');
  protected readonly errorDeRango = signal<string | null>(null);
  /** Lo que se escribe en el panel del período; sólo vale al darle «Aplicar». */
  protected readonly borradorDesde = signal('');
  protected readonly borradorHasta = signal('');

  protected readonly filas = signal<Ingreso[]>([]);
  protected readonly truncado = signal(false);
  protected readonly cargando = signal(true);
  private ultimoError: string | null = null;
  /** Sólo vale la última respuesta: una que llega tarde, porque se cambió el filtro, se descarta. */
  private turno = 0;

  protected readonly enHistorial = computed(() => this.estado() === 'historial');
  protected readonly textoVia = computed(
    () => this.vias.find((v) => v.codigo === this.via())?.etiqueta ?? 'Todas',
  );
  protected readonly textoUnidad = computed(
    () => this.unidades().find((u) => u.codigo === this.unidad())?.nombre ?? 'Todas',
  );
  /** El rango corto en el botón del período (lo da el núcleo); a mano, «Personalizado». */
  protected readonly textoRango = computed(
    () => this.atajos().find((a) => a.codigo === this.atajo())?.corta ?? 'Personalizado',
  );
  protected readonly idFila = (ingreso: Ingreso): string => `${ingreso.via}-${ingreso.id}`;
  protected readonly camposDeBusqueda = (ingreso: Ingreso): CampoDeTexto[] => [
    { campo: 'IDENTIFICACION', texto: ingreso.identidad },
    { campo: 'NOMBRE', texto: ingreso.nombre },
    { campo: 'EMPRESA', texto: ingreso.procedencia },
  ];

  protected readonly mensajeVacio = computed(() =>
    this.estado() === 'adentro' ? 'Nadie está adentro ahora.' : 'No hay movimientos en este rango.',
  );

  protected readonly columnas = computed<ColDef<Ingreso>[]>(() => {
    const comunes: ColDef<Ingreso>[] = [
      {
        field: 'identidad',
        headerName: 'Cédula / código',
        flex: 1.2,
        minWidth: 130,
        cellRenderer: celdaResaltada<Ingreso>(this.nucleo, 'IDENTIFICACION'),
      },
      {
        field: 'nombre',
        headerName: 'Nombre',
        flex: 2,
        minWidth: 190,
        cellRenderer: celdaResaltada<Ingreso>(this.nucleo, 'NOMBRE'),
      },
      {
        colId: 'via',
        headerName: 'Vía',
        flex: 1,
        minWidth: 110,
        valueGetter: ({ data }) => (data ? etiquetaDeVia(data.via) : ''),
      },
      { field: 'unidad', headerName: 'Unidad', flex: 1, minWidth: 100 },
      {
        field: 'procedencia',
        headerName: 'Empresa / motivo',
        flex: 1.5,
        minWidth: 150,
        cellRenderer: celdaResaltada<Ingreso>(this.nucleo, 'EMPRESA'),
      },
      {
        colId: 'medio',
        headerName: 'Medio',
        flex: 1.2,
        minWidth: 120,
        valueGetter: ({ data }) => (data ? textoMedio(data) : ''),
      },
      {
        colId: 'gafete',
        headerName: 'Gafete',
        flex: 0.7,
        minWidth: 80,
        valueGetter: ({ data }) => (data ? textoGafete(data) : ''),
      },
      {
        colId: 'entrada',
        headerName: 'Entró',
        flex: 1.4,
        minWidth: 160,
        // El orden es por el instante, no por el texto que se ve.
        valueGetter: ({ data }) => data?.entrada ?? '',
        valueFormatter: ({ value }) => fechaHora(String(value)),
      },
      {
        colId: 'entrada_por',
        headerName: 'Dio ingreso',
        flex: 1.3,
        minWidth: 130,
        valueGetter: ({ data }) => (data ? textoOperador(data.entrada_por) : ''),
      },
    ];
    if (!this.enHistorial()) {
      return comunes;
    }
    return [
      ...comunes,
      {
        colId: 'salida',
        headerName: 'Salió',
        flex: 1.4,
        minWidth: 160,
        valueGetter: ({ data }) => data?.salida ?? '',
        valueFormatter: ({ value }) => (value ? fechaHora(String(value)) : 'Adentro'),
        cellClass: ({ value }) => (value ? '' : 'estado-bueno'),
      },
      {
        colId: 'salida_por',
        headerName: 'Dio salida',
        flex: 1.3,
        minWidth: 130,
        valueGetter: ({ data }) => (data?.salida ? textoOperador(data.salida_por) : ''),
      },
    ];
  });

  constructor() {
    void this.iniciar();
    let vueltas = 0;
    const refresco = setInterval(() => {
      if (document.hidden) {
        return;
      }
      // Con el tiempo real los cambios llegan solos: la lectura periódica sólo es una red de
      // seguridad (cada minuto). Sin él, se lee cada 15 segundos.
      if (this.enVivo.estado() === 'en_vivo' && ++vueltas % 4 !== 0) {
        return;
      }
      void this.cargar();
    }, CADA_CUANTO_MS);
    inject(DestroyRef).onDestroy(() => clearInterval(refresco));

    // Cada entrada y cada salida (de cualquier unidad) llega como un hecho: se vuelve a leer.
    merge(
      this.enVivo.cambios.pipe(filter((cambio) => cambio.tabla === 'hecho')),
      this.enVivo.reconectado,
    )
      .pipe(debounceTime(300), takeUntilDestroyed())
      .subscribe(() => void this.cargar());
  }

  protected elegirEstado(estado: Estado): void {
    this.estado.set(estado);
    void this.cargar();
  }

  protected elegirVia(via: ViaDeIngreso | ''): void {
    this.via.set(via);
    void this.cargar();
  }

  protected elegirUnidad(unidad: string): void {
    this.unidad.set(unidad);
    void this.cargar();
  }

  /** Un acceso rápido: el rango lo trae el núcleo. */
  protected elegirAtajo(codigo: string): void {
    this.atajo.set(codigo);
    const atajo = this.atajos().find((a) => a.codigo === codigo);
    if (atajo) {
      this.desde.set(atajo.desde ?? '');
      this.hasta.set(atajo.hasta ?? '');
    }
    void this.cargar();
  }

  /** Al abrir el panel del período, las fechas escritas parten de las vigentes. */
  protected abrirRango(): void {
    this.borradorDesde.set(this.desde());
    this.borradorHasta.set(this.hasta());
  }

  protected escribirDesde(evento: Event): void {
    this.borradorDesde.set((evento.target as HTMLInputElement).value);
  }

  protected escribirHasta(evento: Event): void {
    this.borradorHasta.set((evento.target as HTMLInputElement).value);
  }

  /** Un rango escrito a mano; si no vale, el núcleo lo dice al pedirlo. */
  protected aplicarRango(): void {
    this.atajo.set(PERSONALIZADO);
    this.desde.set(this.borradorDesde());
    this.hasta.set(this.borradorHasta());
    void this.cargar();
  }

  /** Los accesos rápidos y su rango por omisión los da el núcleo; las unidades, la nube. */
  private async iniciar(): Promise<void> {
    try {
      const [{ por_omision, atajos }, unidades] = await Promise.all([
        this.api.atajos(),
        this.api.unidades(),
      ]);
      this.atajos.set(atajos);
      this.unidades.set(unidades);
      const inicial = atajos.find((a) => a.codigo === por_omision);
      this.atajo.set(inicial?.codigo ?? '');
      this.desde.set(inicial?.desde ?? '');
      this.hasta.set(inicial?.hasta ?? '');
    } catch (error) {
      this.avisarError(error);
    }
    await this.cargar();
  }

  private async cargar(): Promise<void> {
    const mio = ++this.turno;
    try {
      const lista = await this.api.listar({
        estado: this.estado(),
        via: this.via(),
        unidad: this.unidad(),
        // Lo que está pasando no mira fechas.
        desde: this.enHistorial() ? this.desde() : '',
        hasta: this.enHistorial() ? this.hasta() : '',
      });
      if (mio !== this.turno) {
        return;
      }
      this.filas.set(lista.ingresos);
      this.truncado.set(lista.truncado);
      this.errorDeRango.set(null);
      this.ultimoError = null;
    } catch (error) {
      if (mio !== this.turno) {
        return;
      }
      if (error instanceof ErrorPanel && error.problemas[0]) {
        // El rango no vale: el mensaje es del núcleo y se muestra junto a las fechas.
        this.errorDeRango.set(error.problemas[0].mensaje);
      } else {
        this.avisarError(error);
      }
    } finally {
      if (mio === this.turno) {
        this.cargando.set(false);
      }
    }
  }

  /** Avisa un fallo, sin repetir el mismo aviso en cada lectura periódica. */
  private avisarError(error: unknown): void {
    const mensaje =
      error instanceof ErrorPanel && error.codigo === 'nube'
        ? 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.'
        : 'No se pudo leer los ingresos. Intente de nuevo.';
    if (mensaje !== this.ultimoError) {
      this.avisos.error(mensaje);
    }
    this.ultimoError = mensaje;
  }
}
