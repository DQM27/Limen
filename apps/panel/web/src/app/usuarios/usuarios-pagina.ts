import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  signal,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import type { ColDef, ICellRendererParams } from 'ag-grid-community';
import { Avisos } from '../compartido/avisos';
import { DialogoConfirmar, type DatosConfirmar } from '../compartido/dialogo-confirmar';
import { celdaResaltada } from '../compartido/resaltado';
import { Tabla } from '../compartido/tabla';
import { Nucleo, type CampoDeTexto } from '../nucleo/nucleo';
import { ErrorPanel, type Usuario } from '../panel';
import { ApiUsuarios } from './api-usuarios';
import { DialogoClaveTemporal, type DatosClaveTemporal } from './dialogo-clave-temporal';
import {
  DialogoUsuario,
  type DatosDialogoUsuario,
  type ResultadoDialogoUsuario,
} from './dialogo-usuario';

const CADA_CUANTO_MS = 15_000;

/**
 * Usuarios: quienes operan el sistema (bloque L). Son globales: se crean y se administran
 * aquí, en la nube; los equipos sólo los bajan para iniciar sesión. Crear, editar la cédula y
 * el nombre, activar o desactivar con un interruptor (un usuario nunca se borra: desactivado
 * sólo deja de poder iniciar sesión, y se vuelve a activar cuando se quiera) y restablecer la
 * clave. La pantalla no decide nada: el núcleo normaliza, valida y busca; el Worker guarda.
 */
@Component({
  selector: 'app-usuarios-pagina',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatIconModule, Tabla],
  templateUrl: './usuarios-pagina.html',
  styleUrl: './usuarios-pagina.scss',
})
export class UsuariosPagina {
  private readonly dialogos = inject(MatDialog);
  private readonly nucleo = inject(Nucleo);
  private readonly api = inject(ApiUsuarios);

  protected readonly filas = signal<Usuario[]>([]);
  protected readonly cargando = signal(true);
  private readonly avisos = inject(Avisos);
  /** El último error que se avisó: si sigue igual (el refresco cada 15 s) no se repite. */
  private ultimoError: string | null = null;

  protected readonly idFila = (usuario: Usuario): string => usuario.id;
  protected readonly camposDeBusqueda = (usuario: Usuario): CampoDeTexto[] => [
    { campo: 'IDENTIFICACION', texto: usuario.cedula },
    { campo: 'NOMBRE', texto: usuario.nombre },
  ];

  protected readonly columnas = computed<ColDef<Usuario>[]>(() => [
    {
      field: 'cedula',
      headerName: 'Cédula',
      flex: 1,
      minWidth: 130,
      cellRenderer: celdaResaltada<Usuario>(this.nucleo, 'IDENTIFICACION'),
    },
    {
      field: 'nombre',
      headerName: 'Nombre',
      flex: 2,
      minWidth: 180,
      cellRenderer: celdaResaltada<Usuario>(this.nucleo, 'NOMBRE'),
    },
    {
      field: 'activo',
      headerName: 'Activo',
      // Un sí/no sería una casilla en AG Grid: aquí es un interruptor.
      cellDataType: false,
      flex: 0.8,
      minWidth: 100,
      cellRenderer: (parametros: ICellRendererParams<Usuario>) => this.interruptor(parametros),
    },
    {
      field: 'debe_cambiar_clave',
      headerName: 'Clave',
      cellDataType: false,
      flex: 1,
      minWidth: 120,
      valueFormatter: ({ value }) => (value ? 'Temporal' : 'Propia'),
      cellClass: ({ value }) => (value ? 'estado-aviso' : 'estado-neutro'),
    },
    {
      colId: 'acciones',
      headerName: '',
      flex: 2.2,
      minWidth: 280,
      sortable: false,
      cellRenderer: (parametros: ICellRendererParams<Usuario>) => this.acciones(parametros),
    },
  ]);

  constructor() {
    void this.cargar();
    const refresco = setInterval(() => {
      if (!document.hidden) {
        void this.cargar();
      }
    }, CADA_CUANTO_MS);
    inject(DestroyRef).onDestroy(() => clearInterval(refresco));
  }

  protected nuevo(): void {
    this.abrirDialogo({});
  }

  private editar(usuario: Usuario): void {
    this.abrirDialogo({ usuario });
  }

  private abrirDialogo(datos: DatosDialogoUsuario): void {
    this.dialogos
      .open<DialogoUsuario, DatosDialogoUsuario, ResultadoDialogoUsuario>(DialogoUsuario, {
        data: datos,
        autoFocus: 'dialog',
      })
      .afterClosed()
      .subscribe((resultado) => {
        if (resultado?.tipo === 'editado') {
          this.avisos.exito(`Se guardaron los cambios de ${resultado.usuario.nombre}.`);
        }
        if (resultado?.tipo === 'creado') {
          const { usuario, clave_temporal } = resultado.creado;
          this.avisos.exito(
            `${usuario.nombre} ya puede iniciar sesión con su clave temporal.`,
            'Usuario creado',
          );
          this.mostrarClave({
            cedula: usuario.cedula,
            nombre: usuario.nombre,
            clave: clave_temporal,
          });
        }
        if (resultado) {
          void this.cargar();
        }
      });
  }

  private mostrarClave(datos: DatosClaveTemporal): void {
    this.dialogos.open(DialogoClaveTemporal, { data: datos, disableClose: true });
  }

  private restablecerClave(usuario: Usuario): void {
    this.confirmar(
      {
        titulo: 'Restablecer la clave',
        mensaje: `¿Dar a «${usuario.nombre}» una clave temporal nueva? La que tiene deja de servir y tendrá que cambiar la nueva al entrar.`,
        confirmar: 'Restablecer',
      },
      async () => {
        const clave = await this.api.restablecerClave(usuario.id);
        this.avisos.exito(
          `${usuario.nombre} tiene una clave temporal nueva.`,
          'Clave restablecida',
        );
        this.mostrarClave({ cedula: usuario.cedula, nombre: usuario.nombre, clave });
        await this.cargar();
      },
    );
  }

  /**
   * Activar o desactivar: sólo cambia «activo»; a nadie se le borra nada y se revierte con otro
   * clic. Desactivar pide confirmación (deja a alguien sin entrar); activar, no.
   */
  private cambiarActivo(usuario: Usuario): void {
    const activo = !usuario.activo;
    const guardar = async (): Promise<void> => {
      await this.api.editar(usuario.id, {
        cedula: usuario.cedula,
        nombre: usuario.nombre,
        activo,
      });
      this.avisos.exito(
        activo
          ? `${usuario.nombre} puede iniciar sesión otra vez.`
          : `${usuario.nombre} ya no puede iniciar sesión.`,
        activo ? 'Usuario activado' : 'Usuario desactivado',
      );
      await this.cargar();
    };
    if (activo) {
      guardar().catch((error: unknown) => this.fallo(this.mensaje(error)));
      return;
    }
    this.confirmar(
      {
        titulo: 'Desactivar usuario',
        mensaje: `¿Desactivar a «${usuario.nombre}»? Ya no podrá iniciar sesión, pero no se borra nada: lo que registró se conserva y lo puede volver a activar cuando quiera.`,
        confirmar: 'Desactivar',
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
          accion().catch((error: unknown) => this.fallo(this.mensaje(error)));
        }
      });
  }

  /** Los botones de la fila. */
  private acciones(parametros: ICellRendererParams<Usuario>): HTMLElement {
    const contenedor = document.createElement('div');
    contenedor.className = 'acciones-de-fila';
    const usuario = parametros.data;
    if (!usuario) {
      return contenedor;
    }
    const boton = (texto: string, clase: string, alHacerClic: () => void): void => {
      const elemento = document.createElement('button');
      elemento.type = 'button';
      elemento.className = `boton-celda ${clase}`.trim();
      elemento.textContent = texto;
      elemento.addEventListener('click', alHacerClic);
      contenedor.append(elemento);
    };
    boton('Editar', '', () => this.editar(usuario));
    boton('Clave', '', () => this.restablecerClave(usuario));
    return contenedor;
  }

  /** El interruptor de la fila: encendido es «activo». No cambia solo: espera a que la nube responda. */
  private interruptor(parametros: ICellRendererParams<Usuario>): HTMLElement {
    const usuario = parametros.data;
    const boton = document.createElement('button');
    if (!usuario) {
      return boton;
    }
    boton.type = 'button';
    boton.className = 'interruptor';
    boton.setAttribute('role', 'switch');
    boton.setAttribute('aria-checked', String(usuario.activo));
    boton.setAttribute(
      'aria-label',
      `${usuario.activo ? 'Desactivar' : 'Activar'} a ${usuario.nombre}`,
    );
    boton.append(document.createElement('span'));
    boton.addEventListener('click', () => this.cambiarActivo(usuario));
    return boton;
  }

  private async cargar(): Promise<void> {
    try {
      this.filas.set(await this.api.listar());
      this.ultimoError = null;
    } catch (error) {
      this.fallo(this.mensaje(error));
    } finally {
      this.cargando.set(false);
    }
  }

  private fallo(mensaje: string): void {
    if (mensaje !== this.ultimoError) {
      this.avisos.error(mensaje);
    }
    this.ultimoError = mensaje;
  }

  private mensaje(error: unknown): string {
    const codigo = error instanceof ErrorPanel ? error.codigo : '';
    return (
      this.nucleo.mensajeDeError(codigo) ||
      (codigo === 'nube'
        ? 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.'
        : 'No se pudo completar la operación. Intente de nuevo.')
    );
  }
}
