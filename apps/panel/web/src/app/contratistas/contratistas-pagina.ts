import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  signal,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import type { ColDef, ICellRendererParams } from 'ag-grid-community';
import { debounceTime, filter, merge } from 'rxjs';
import { Avisos } from '../compartido/avisos';
import { DialogoConfirmar, type DatosConfirmar } from '../compartido/dialogo-confirmar';
import { soloFecha } from '../compartido/fecha';
import { celdaResaltada } from '../compartido/resaltado';
import { Tabla } from '../compartido/tabla';
import { EnVivo } from '../en-vivo/en-vivo';
import { Nucleo, type CampoDeTexto } from '../nucleo/nucleo';
import { ErrorPanel, type Contratista, type EmpresaCatalogo } from '../panel';
import { ApiContratistas } from './api-contratistas';
import {
  DialogoContratista,
  type DatosDialogoContratista,
  type ResultadoDialogoContratista,
} from './dialogo-contratista';
import { etiquetaDeTipo } from './etiquetas';

const CADA_CUANTO_MS = 15_000;

/**
 * Contratistas: el catálogo global de quienes entran a trabajar (bloque B). Se registran y se
 * editan aquí, en la nube; los equipos lo bajan solos. Nunca se borra a nadie: a quien ya no
 * debe entrar se le quita el acceso (y se le puede devolver). La pantalla no decide nada: el
 * núcleo normaliza, valida, busca y dice si un PRAIND está vencido; el Worker guarda.
 */
@Component({
  selector: 'app-contratistas-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatIconModule, Tabla],
  templateUrl: './contratistas-pagina.html',
  styleUrl: './contratistas-pagina.scss',
})
export class ContratistasPagina {
  private readonly dialogos = inject(MatDialog);
  private readonly nucleo = inject(Nucleo);
  private readonly api = inject(ApiContratistas);
  private readonly enVivo = inject(EnVivo);
  private readonly avisos = inject(Avisos);

  protected readonly filas = signal<Contratista[]>([]);
  private readonly empresas = signal<EmpresaCatalogo[]>([]);
  protected readonly cargando = signal(true);
  /** El último error que se avisó: si sigue igual (el refresco cada 15 s) no se repite. */
  private ultimoError: string | null = null;

  protected readonly idFila = (contratista: Contratista): string => contratista.id;
  protected readonly camposDeBusqueda = (contratista: Contratista): CampoDeTexto[] => [
    { campo: 'IDENTIFICACION', texto: contratista.cedula },
    { campo: 'NOMBRE', texto: contratista.nombre },
    { campo: 'EMPRESA', texto: contratista.empresa_nombre },
  ];

  protected readonly columnas = computed<ColDef<Contratista>[]>(() => [
    {
      field: 'cedula',
      headerName: 'Cédula',
      flex: 1,
      minWidth: 120,
      cellRenderer: celdaResaltada<Contratista>(this.nucleo, 'IDENTIFICACION'),
    },
    {
      field: 'nombre',
      headerName: 'Nombre',
      flex: 2,
      minWidth: 180,
      cellRenderer: celdaResaltada<Contratista>(this.nucleo, 'NOMBRE'),
    },
    {
      field: 'empresa_nombre',
      headerName: 'Empresa',
      flex: 1.5,
      minWidth: 150,
      cellRenderer: celdaResaltada<Contratista>(this.nucleo, 'EMPRESA'),
    },
    {
      field: 'tipo_ingreso',
      headerName: 'Tipo',
      flex: 0.8,
      minWidth: 100,
      valueFormatter: ({ value }) => etiquetaDeTipo(value),
    },
    {
      field: 'fecha_vencimiento_praind',
      headerName: 'Vence PRAIND',
      flex: 1,
      minWidth: 120,
      valueFormatter: ({ value }) => soloFecha(value),
      // Si está vencido lo dice el núcleo; la celda sólo se pinta.
      cellClass: ({ value }) =>
        this.nucleo.praindVencido(String(value), Date.now()) ? 'estado-malo' : 'estado-bueno',
    },
    {
      field: 'tiene_acceso',
      headerName: 'Acceso',
      // Un sí/no sería una casilla en AG Grid: aquí es un interruptor.
      cellDataType: false,
      flex: 0.7,
      minWidth: 90,
      cellRenderer: (parametros: ICellRendererParams<Contratista>) => this.interruptor(parametros),
    },
    {
      colId: 'acciones',
      headerName: '',
      flex: 0.8,
      minWidth: 100,
      sortable: false,
      cellRenderer: (parametros: ICellRendererParams<Contratista>) => this.acciones(parametros),
    },
  ]);

  constructor() {
    void this.cargar();
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

    // Cuando la base avisa que cambió un contratista o una empresa (aquí o en un equipo), se
    // vuelve a leer al instante.
    merge(
      this.enVivo.cambios.pipe(
        filter((cambio) => cambio.tabla === 'contratista' || cambio.tabla === 'empresa'),
      ),
      this.enVivo.reconectado,
    )
      .pipe(debounceTime(300), takeUntilDestroyed())
      .subscribe(() => void this.cargar());
  }

  protected nuevo(): void {
    this.abrirDialogo({ empresas: this.empresas() });
  }

  private editar(contratista: Contratista): void {
    this.abrirDialogo({ contratista, empresas: this.empresas() });
  }

  private abrirDialogo(datos: DatosDialogoContratista): void {
    this.dialogos
      .open<DialogoContratista, DatosDialogoContratista, ResultadoDialogoContratista>(
        DialogoContratista,
        { data: datos, autoFocus: 'dialog' },
      )
      .afterClosed()
      .subscribe((resultado) => {
        if (!resultado) {
          return;
        }
        this.avisos.exito(
          resultado.tipo === 'registrado'
            ? `${resultado.contratista.nombre} quedó registrado.`
            : `Se guardaron los cambios de ${resultado.contratista.nombre}.`,
          resultado.tipo === 'registrado' ? 'Contratista registrado' : 'Contratista editado',
        );
        void this.cargar();
      });
  }

  /**
   * Dar o quitar el acceso: sólo cambia «tiene acceso»; a nadie se le borra nada y se revierte
   * con otro clic. Quitarlo pide confirmación (deja a alguien sin entrar); devolverlo, no. Con el
   * PRAIND vencido se le puede quitar el acceso: la regla es del núcleo.
   */
  private cambiarAcceso(contratista: Contratista): void {
    const tieneAcceso = !contratista.tiene_acceso;
    const guardar = async (): Promise<void> => {
      await this.api.editar(contratista.id, {
        cedula: contratista.cedula,
        nombre: contratista.nombre,
        empresa_id: contratista.empresa_id,
        tipo_ingreso: contratista.tipo_ingreso,
        fecha_vencimiento_praind: contratista.fecha_vencimiento_praind,
        tiene_acceso: tieneAcceso,
      });
      this.avisos.exito(
        tieneAcceso
          ? `${contratista.nombre} puede entrar otra vez.`
          : `${contratista.nombre} ya no puede entrar.`,
        tieneAcceso ? 'Acceso devuelto' : 'Acceso quitado',
      );
      await this.cargar();
    };
    if (tieneAcceso) {
      guardar().catch((error: unknown) => this.avisarError(error));
      return;
    }
    this.confirmar(
      {
        titulo: 'Quitar el acceso',
        mensaje: `¿Quitar el acceso a «${contratista.nombre}»? No podrá entrar, pero no se borra nada: se le puede devolver cuando se quiera.`,
        confirmar: 'Quitar acceso',
      },
      guardar,
    );
  }

  private confirmar(datos: DatosConfirmar, accion: () => Promise<void>): void {
    this.dialogos
      .open(DialogoConfirmar, { data: datos })
      .afterClosed()
      .subscribe((confirmado) => {
        if (confirmado) {
          accion().catch((error: unknown) => this.avisarError(error));
        }
      });
  }

  /** Los botones de la fila. */
  private acciones(parametros: ICellRendererParams<Contratista>): HTMLElement {
    const contenedor = document.createElement('div');
    contenedor.className = 'acciones-de-fila';
    const contratista = parametros.data;
    if (!contratista) {
      return contenedor;
    }
    const boton = document.createElement('button');
    boton.type = 'button';
    boton.className = 'boton-celda';
    boton.textContent = 'Editar';
    boton.addEventListener('click', () => this.editar(contratista));
    contenedor.append(boton);
    return contenedor;
  }

  /** El interruptor de la fila: encendido es «tiene acceso». No cambia solo: espera a la nube. */
  private interruptor(parametros: ICellRendererParams<Contratista>): HTMLElement {
    const contratista = parametros.data;
    const boton = document.createElement('button');
    if (!contratista) {
      return boton;
    }
    boton.type = 'button';
    boton.className = 'interruptor';
    boton.setAttribute('role', 'switch');
    boton.setAttribute('aria-checked', String(contratista.tiene_acceso));
    boton.setAttribute(
      'aria-label',
      `${contratista.tiene_acceso ? 'Quitar' : 'Dar'} el acceso a ${contratista.nombre}`,
    );
    boton.append(document.createElement('span'));
    boton.addEventListener('click', () => this.cambiarAcceso(contratista));
    return boton;
  }

  private async cargar(): Promise<void> {
    try {
      const { contratistas, empresas } = await this.api.listar();
      this.filas.set(contratistas);
      this.empresas.set(empresas);
      this.ultimoError = null;
    } catch (error) {
      this.avisarError(error, true);
    } finally {
      this.cargando.set(false);
    }
  }

  /** Avisa un fallo; en la lectura periódica no se repite el mismo aviso. */
  private avisarError(error: unknown, deLectura = false): void {
    const mensaje = this.mensaje(error);
    if (!deLectura || mensaje !== this.ultimoError) {
      this.avisos.error(mensaje);
    }
    if (deLectura) {
      this.ultimoError = mensaje;
    }
  }

  private mensaje(error: unknown): string {
    if (error instanceof ErrorPanel && error.problemas[0]) {
      return error.problemas[0].mensaje;
    }
    return error instanceof ErrorPanel && error.codigo === 'nube'
      ? 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.'
      : 'No se pudo completar la operación. Intente de nuevo.';
  }
}
