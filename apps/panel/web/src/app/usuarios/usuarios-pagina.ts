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
 * el nombre, dar de baja y reactivar (nunca se borra a nadie) y restablecer la clave. La
 * pantalla no decide nada: el núcleo normaliza, valida y busca; el Worker guarda.
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
  protected readonly error = signal<string | null>(null);

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
      headerName: 'Estado',
      // Un sí/no sería una casilla en AG Grid: se muestra el texto.
      cellDataType: false,
      flex: 1,
      minWidth: 110,
      valueFormatter: ({ value }) => (value ? 'Activo' : 'De baja'),
      cellClass: ({ value }) => (value ? 'estado-bueno' : 'estado-malo'),
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
        if (resultado?.tipo === 'creado') {
          const { usuario, clave_temporal } = resultado.creado;
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
        this.mostrarClave({ cedula: usuario.cedula, nombre: usuario.nombre, clave });
        await this.cargar();
      },
    );
  }

  /** Dar de baja o reactivar: sólo cambia «activo»; a nadie se le borra nada. */
  private cambiarEstado(usuario: Usuario): void {
    const activo = !usuario.activo;
    this.confirmar(
      {
        titulo: activo ? 'Reactivar usuario' : 'Dar de baja',
        mensaje: activo
          ? `¿Reactivar a «${usuario.nombre}»? Podrá volver a iniciar sesión.`
          : `¿Dar de baja a «${usuario.nombre}»? Ya no podrá iniciar sesión, pero lo que registró se conserva.`,
        confirmar: activo ? 'Reactivar' : 'Dar de baja',
      },
      async () => {
        await this.api.editar(usuario.id, {
          cedula: usuario.cedula,
          nombre: usuario.nombre,
          activo,
        });
        await this.cargar();
      },
    );
  }

  private confirmar(datos: DatosConfirmar, accion: () => Promise<void>): void {
    this.dialogos
      .open(DialogoConfirmar, { data: datos })
      .afterClosed()
      .subscribe((confirmado) => {
        if (confirmado) {
          accion().catch((error: unknown) => this.error.set(this.mensaje(error)));
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
    boton(usuario.activo ? 'Dar de baja' : 'Reactivar', usuario.activo ? 'peligro' : '', () =>
      this.cambiarEstado(usuario),
    );
    return contenedor;
  }

  private async cargar(): Promise<void> {
    try {
      this.filas.set(await this.api.listar());
      this.error.set(null);
    } catch (error) {
      this.error.set(this.mensaje(error));
    } finally {
      this.cargando.set(false);
    }
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
