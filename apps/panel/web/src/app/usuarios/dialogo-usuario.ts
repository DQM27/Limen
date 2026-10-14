import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { Nucleo } from '../nucleo/nucleo';
import { ErrorPanel, type Usuario, type UsuarioCreado } from '../panel';
import { ApiUsuarios } from './api-usuarios';

export interface DatosDialogoUsuario {
  /** Sin usuario, es un alta; con usuario, una edición de su cédula y su nombre. */
  usuario?: Usuario;
}

/** Lo que el diálogo devuelve al cerrarse: el usuario nuevo con su clave, o el editado. */
export type ResultadoDialogoUsuario =
  { tipo: 'creado'; creado: UsuarioCreado } | { tipo: 'editado'; usuario: Usuario };

// Sólo lo que no es del núcleo: la nube, el panel. Los mensajes de cédula, nombre y cédula
// repetida los da el núcleo por su código.
const MENSAJE_DEL_PANEL: Record<string, string> = {
  nube: 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.',
  no_encontrado: 'Ese usuario ya no existe.',
};

/**
 * Alta o edición de un usuario. La pantalla no normaliza ni valida: lo que se escribe pasa por
 * el núcleo (mayúscula del nombre al escribir, cédula normalizada, mensajes de error) y aquí sólo
 * se muestra lo que responde. La cédula repetida la decide el núcleo en el Worker, con lo que
 * tiene la nube.
 */
@Component({
  selector: 'app-dialogo-usuario',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatButtonModule, MatDialogModule, MatFormFieldModule, MatInputModule],
  templateUrl: './dialogo-usuario.html',
  styleUrl: './dialogo-usuario.scss',
})
export class DialogoUsuario {
  private readonly dialogo =
    inject<MatDialogRef<DialogoUsuario, ResultadoDialogoUsuario>>(MatDialogRef);
  private readonly nucleo = inject(Nucleo);
  private readonly api = inject(ApiUsuarios);
  protected readonly usuario = inject<DatosDialogoUsuario>(MAT_DIALOG_DATA).usuario;

  protected readonly cedula = signal(this.usuario?.cedula ?? '');
  protected readonly nombre = signal(this.usuario?.nombre ?? '');
  protected readonly enviando = signal(false);
  /** El mensaje de lo que respondió el Worker (cédula repetida, falla de la nube…). */
  protected readonly errorDelServidor = signal<string | null>(null);
  protected readonly intentado = signal(false);

  /** Cómo quedarían la cédula y el nombre según el núcleo, o por qué no valen. */
  protected readonly resultado = computed(() =>
    this.nucleo.datosDeUsuario(this.cedula(), this.nombre()),
  );

  /** El error del núcleo, una vez que hay algo escrito en los dos campos o se intentó guardar. */
  protected readonly errorDelNucleo = computed(() => {
    const resultado = this.resultado();
    const hayAlgo = this.cedula().trim() !== '' && this.nombre().trim() !== '';
    return !resultado.ok && (this.intentado() || hayAlgo) ? resultado.mensaje : null;
  });

  protected readonly vistaPrevia = computed(() => {
    const resultado = this.resultado();
    return resultado.ok ? resultado.datos : null;
  });

  protected escribirCedula(evento: Event): void {
    this.cedula.set((evento.target as HTMLInputElement).value);
    this.errorDelServidor.set(null);
  }

  /**
   * Mientras se escribe, el nombre se ve como quedará (mayúscula): la forma la da el núcleo. El
   * cursor se queda donde estaba.
   */
  protected escribirNombre(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const cursor = campo.selectionStart;
    const queda = this.nucleo.nombreAlEscribir(campo.value);
    if (queda !== campo.value) {
      campo.value = queda;
      campo.setSelectionRange(cursor, cursor);
    }
    this.nombre.set(queda);
    this.errorDelServidor.set(null);
  }

  protected async guardar(evento: Event): Promise<void> {
    evento.preventDefault();
    this.intentado.set(true);
    if (this.enviando() || !this.resultado().ok) {
      return;
    }
    this.enviando.set(true);
    this.errorDelServidor.set(null);
    const datos = { cedula: this.cedula(), nombre: this.nombre() };
    try {
      if (this.usuario) {
        const editado = await this.api.editar(this.usuario.id, {
          ...datos,
          activo: this.usuario.activo,
        });
        this.dialogo.close({ tipo: 'editado', usuario: editado });
      } else {
        this.dialogo.close({ tipo: 'creado', creado: await this.api.crear(datos) });
      }
    } catch (error) {
      const codigo = error instanceof ErrorPanel ? error.codigo : '';
      this.errorDelServidor.set(
        this.nucleo.mensajeDeError(codigo) ||
          MENSAJE_DEL_PANEL[codigo] ||
          'No se pudo completar la operación. Intente de nuevo.',
      );
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }
}
