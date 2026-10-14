import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  signal,
} from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import {
  ErrorPanel,
  registrarDispositivo,
  type CodigoEmitido,
  type TipoDispositivo,
  type Unidad,
} from '../panel';
import { ETIQUETA_TIPO } from './filas';

export interface DatosDialogoEquipo {
  unidades: readonly Unidad[];
}

const MENSAJE_ERROR: Record<string, string> = {
  datos_invalidos: 'Revise el nombre, la unidad y el tipo del equipo.',
  nube: 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.',
  unidad_inexistente: 'Esa unidad operativa no existe. Elija una de la lista.',
};

const TIPOS = (Object.keys(ETIQUETA_TIPO) as TipoDispositivo[]).map((valor) => ({
  valor,
  etiqueta: valor === 'visor' ? 'Visor (sólo lee)' : (ETIQUETA_TIPO[valor] ?? valor),
}));

/**
 * Registrar un equipo y mostrar su código de vinculación. Dos momentos en la misma
 * ventana: el formulario y, una vez emitido, el código (que se muestra una sola vez).
 * Se cierra con `true` si se registró algún equipo, para que la tabla se refresque.
 */
@Component({
  selector: 'app-dialogo-equipo',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    MatSelectModule,
  ],
  templateUrl: './dialogo-equipo.html',
  styleUrl: './dialogo-equipo.scss',
})
export class DialogoEquipo {
  private readonly dialogo = inject<MatDialogRef<DialogoEquipo, boolean>>(MatDialogRef);
  protected readonly unidades = inject<DatosDialogoEquipo>(MAT_DIALOG_DATA).unidades;
  protected readonly tipos = TIPOS;

  protected readonly nombre = signal('');
  // Con una sola unidad no hay nada que decidir: queda elegida.
  protected readonly unidad = signal(
    this.unidades.length === 1 ? (this.unidades[0]?.codigo ?? '') : '',
  );
  protected readonly tipo = signal<TipoDispositivo>('pc');
  protected readonly enviando = signal(false);
  protected readonly error = signal<string | null>(null);

  /** El código recién emitido, con el momento en que vence. */
  protected readonly emitido = signal<(CodigoEmitido & { vence: number }) | null>(null);
  private readonly ahora = signal(Date.now());
  protected readonly segundosRestantes = computed(() => {
    const codigo = this.emitido();
    return codigo ? Math.max(0, Math.floor((codigo.vence - this.ahora()) / 1000)) : 0;
  });
  private registrado = false;

  constructor() {
    const reloj = setInterval(() => this.ahora.set(Date.now()), 1000);
    inject(DestroyRef).onDestroy(() => clearInterval(reloj));
  }

  protected tiempo(segundos: number): string {
    const resto = String(segundos % 60).padStart(2, '0');
    return `${Math.floor(segundos / 60)}:${resto}`;
  }

  protected escribir(evento: Event): void {
    this.nombre.set((evento.target as HTMLInputElement).value);
  }

  protected async registrar(evento: Event): Promise<void> {
    evento.preventDefault();
    if (this.enviando()) {
      return;
    }
    this.enviando.set(true);
    this.error.set(null);
    try {
      const codigo = await registrarDispositivo({
        nombre: this.nombre(),
        unidad: this.unidad(),
        tipo: this.tipo(),
      });
      this.registrado = true;
      this.emitido.set({ ...codigo, vence: Date.now() + codigo.vigencia_minutos * 60_000 });
    } catch (error) {
      const codigo = error instanceof ErrorPanel ? error.codigo : '';
      this.error.set(
        MENSAJE_ERROR[codigo] ?? 'No se pudo completar la operación. Intente de nuevo.',
      );
    } finally {
      this.enviando.set(false);
    }
  }

  protected cerrar(): void {
    this.dialogo.close(this.registrado);
  }
}
