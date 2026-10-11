import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  type OnInit,
  signal,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import type { ColDef } from 'ag-grid-community';
import { soloFecha, soloHora } from '../compartido/formato';
import { Tabla } from '../compartido/tabla/tabla';
import { etiquetaDeVia, textoGafete, textoMedio, textoOperador } from '../dentro/etiquetas';
import { atajosDeFecha, listarHistorial } from '../nucleo/comandos';
import { esErrorApp } from '../nucleo/tauri';
import type { AtajoFecha, Historial, Movimiento } from '../nucleo/tipos';
import { CambiosDeLaNube } from '../nucleo/cambios-de-la-nube';

/** El campo del rango al que el núcleo atribuye un error. */
type CampoFecha = 'desde' | 'hasta';

/**
 * Las entradas y salidas de un rango de fechas, de la más reciente a la más
 * antigua. Los accesos rápidos (hoy, esta semana…) y su rango por omisión los
 * da el núcleo; aquí sólo se muestran y se escribe el rango a mano si hace
 * falta. Si el rango trae demasiados movimientos, el núcleo entrega los más
 * recientes y lo avisa.
 */
@Component({
  selector: 'app-historial-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatIconModule, MatMenuModule, Tabla],
  templateUrl: './historial-pagina.html',
  styleUrl: './historial-pagina.scss',
})
export class HistorialPagina implements OnInit {
  protected readonly atajos = signal<AtajoFecha[]>([]);
  /** El acceso rápido elegido; `null` cuando el rango se escribió a mano. */
  protected readonly atajo = signal<AtajoFecha | null>(null);
  /** `AAAA-MM-DD`; vacío = sin límite. */
  protected readonly desde = signal('');
  protected readonly hasta = signal('');
  protected readonly historial = signal<Historial | null>(null);
  protected readonly cargando = signal(true);
  protected readonly error = signal<string | null>(null);
  protected readonly campoConError = signal<CampoFecha | null>(null);

  protected readonly movimientos = computed(() => this.historial()?.movimientos ?? []);
  protected readonly textoAtajo = computed(() => this.atajo()?.corta ?? 'Personalizado');

  protected readonly idFila = (fila: Movimiento): string => `${fila.via}-${fila.ingreso_id}`;

  protected readonly columnas: ColDef<Movimiento>[] = [
    { field: 'identidad', headerName: 'Cédula / código', flex: 1.2, minWidth: 130 },
    { field: 'nombre', headerName: 'Nombre', flex: 2, minWidth: 200 },
    {
      colId: 'via',
      headerName: 'Vía',
      flex: 1,
      minWidth: 110,
      valueGetter: (p) => (p.data ? etiquetaDeVia(p.data.via) : ''),
    },
    { field: 'procedencia', headerName: 'Empresa / motivo', flex: 1.5, minWidth: 150 },
    {
      colId: 'medio',
      headerName: 'Medio',
      flex: 1.2,
      minWidth: 130,
      valueGetter: (p) => (p.data ? textoMedio(p.data) : ''),
    },
    {
      colId: 'gafete',
      headerName: 'Gafete',
      flex: 0.8,
      minWidth: 90,
      valueGetter: (p) => (p.data ? textoGafete(p.data) : ''),
    },
    {
      colId: 'fecha_entrada',
      headerName: 'Entró (fecha)',
      flex: 1,
      minWidth: 120,
      valueGetter: (p) => (p.data ? soloFecha(p.data.entrada) : ''),
    },
    {
      colId: 'hora_entrada',
      headerName: 'Entró (hora)',
      flex: 0.8,
      minWidth: 100,
      valueGetter: (p) => (p.data ? soloHora(p.data.entrada) : ''),
    },
    {
      colId: 'dio_ingreso',
      headerName: 'Dio ingreso',
      flex: 1.3,
      minWidth: 140,
      valueGetter: (p) => (p.data ? textoOperador(p.data.entrada_por) : ''),
    },
    {
      colId: 'fecha_salida',
      headerName: 'Salió (fecha)',
      flex: 1,
      minWidth: 120,
      valueGetter: (p) => (p.data ? (p.data.salida ? soloFecha(p.data.salida) : 'Adentro') : ''),
    },
    {
      colId: 'hora_salida',
      headerName: 'Salió (hora)',
      flex: 0.8,
      minWidth: 100,
      valueGetter: (p) => (p.data?.salida ? soloHora(p.data.salida) : ''),
    },
    {
      colId: 'dio_salida',
      headerName: 'Dio salida',
      flex: 1.3,
      minWidth: 140,
      valueGetter: (p) => (p.data?.salida_por ? textoOperador(p.data.salida_por) : ''),
    },
  ];

  constructor() {
    // Lo que registran los demás equipos llega de la nube: se vuelve a cargar.
    inject(CambiosDeLaNube).alLlegar(() => void this.cargar());
  }

  ngOnInit(): void {
    void this.iniciar();
  }

  protected elegirAtajo(atajo: AtajoFecha): void {
    this.atajo.set(atajo);
    this.desde.set(atajo.desde ?? '');
    this.hasta.set(atajo.hasta ?? '');
    void this.cargar();
  }

  /** Una fecha escrita a mano: el rango deja de ser un acceso rápido. */
  protected cambiarFecha(campo: CampoFecha, evento: Event): void {
    const valor = (evento.target as HTMLInputElement).value;
    (campo === 'desde' ? this.desde : this.hasta).set(valor);
    this.atajo.set(null);
    void this.cargar();
  }

  private async iniciar(): Promise<void> {
    try {
      const atajos = await atajosDeFecha();
      this.atajos.set(atajos);
      const inicial = atajos.find((a) => a.por_omision) ?? null;
      if (inicial) {
        this.atajo.set(inicial);
        this.desde.set(inicial.desde ?? '');
        this.hasta.set(inicial.hasta ?? '');
      }
    } catch (error) {
      this.error.set(
        esErrorApp(error) ? error.mensaje : 'No se pudieron cargar los accesos rápidos.',
      );
    }
    await this.cargar();
  }

  private async cargar(): Promise<void> {
    this.cargando.set(true);
    try {
      this.historial.set(await listarHistorial(this.desde() || null, this.hasta() || null));
      this.error.set(null);
      this.campoConError.set(null);
    } catch (error) {
      if (esErrorApp(error)) {
        this.error.set(error.mensaje);
        this.campoConError.set(
          error.campo === 'desde' || error.campo === 'hasta' ? error.campo : null,
        );
      } else {
        this.error.set('No se pudo cargar el historial.');
        this.campoConError.set(null);
      }
    } finally {
      this.cargando.set(false);
    }
  }
}
