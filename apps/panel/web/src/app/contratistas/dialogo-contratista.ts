import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import {
  MatAutocompleteModule,
  type MatAutocompleteSelectedEvent,
} from '@angular/material/autocomplete';
import { MatButtonModule } from '@angular/material/button';
import {
  MAT_DIALOG_DATA,
  MatDialog,
  MatDialogModule,
  MatDialogRef,
} from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import { Nucleo } from '../nucleo/nucleo';
import {
  ErrorPanel,
  type Contratista,
  type EmpresaCatalogo,
  type Problema,
  type TipoIngreso,
} from '../panel';
import { ApiContratistas } from './api-contratistas';
import { DialogoEmpresa } from './dialogo-empresa';
import { etiquetaDeTipo } from './etiquetas';

export interface DatosDialogoContratista {
  /** Sin contratista, es un alta; con contratista, una edición. */
  contratista?: Contratista;
  empresas: EmpresaCatalogo[];
}

/** Lo que el diálogo devuelve al cerrarse: el contratista guardado y si era alta o edición. */
export interface ResultadoDialogoContratista {
  tipo: 'registrado' | 'editado';
  contratista: Contratista;
}

/** El campo del núcleo (`Problema.campo`) → el campo del formulario que lo muestra. */
const CAMPOS = ['cedula', 'nombre', 'empresa_id', 'fecha_vencimiento_praind'] as const;
type Campo = (typeof CAMPOS)[number];

const MENSAJE_DEL_PANEL: Record<string, string> = {
  nube: 'El panel no pudo hablar con la base de datos. Revise los secretos del Worker.',
  no_encontrado: 'Ese contratista ya no existe.',
};

/**
 * Alta o edición de un contratista. La pantalla no valida ni normaliza: el nombre se ve en
 * mayúscula mientras se escribe (lo dice el núcleo) y todo lo que no vale vuelve del núcleo, por
 * el Worker, ya con el campo donde se muestra: se marcan todos a la vez. La empresa se elige de
 * las que existen (se filtran con la búsqueda del núcleo); una nueva se crea aparte, con «+».
 */
@Component({
  selector: 'app-dialogo-contratista',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatAutocompleteModule,
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    MatSelectModule,
    MatSlideToggleModule,
  ],
  templateUrl: './dialogo-contratista.html',
  styleUrl: './dialogo-contratista.scss',
})
export class DialogoContratista {
  private readonly dialogo =
    inject<MatDialogRef<DialogoContratista, ResultadoDialogoContratista>>(MatDialogRef);
  private readonly dialogos = inject(MatDialog);
  private readonly nucleo = inject(Nucleo);
  private readonly api = inject(ApiContratistas);
  private readonly datos = inject<DatosDialogoContratista>(MAT_DIALOG_DATA);
  protected readonly contratista = this.datos.contratista;
  protected readonly etiquetaDeTipo = etiquetaDeTipo;

  protected readonly cedula = signal(this.contratista?.cedula ?? '');
  protected readonly nombre = signal(this.contratista?.nombre ?? '');
  protected readonly empresas = signal(this.datos.empresas);
  /** Lo que se escribe en el campo de empresa, y la empresa que se eligió de la lista. */
  protected readonly empresaTexto = signal(this.contratista?.empresa_nombre ?? '');
  private readonly empresaElegida = signal<EmpresaCatalogo | null>(
    this.contratista
      ? { id: this.contratista.empresa_id, nombre: this.contratista.empresa_nombre }
      : null,
  );
  protected readonly tipo = signal<TipoIngreso>(this.contratista?.tipo_ingreso ?? 'PRAIND');
  protected readonly fecha = signal(this.contratista?.fecha_vencimiento_praind ?? '');
  protected readonly tieneAcceso = signal(this.contratista?.tiene_acceso ?? true);

  protected readonly enviando = signal(false);
  protected readonly problemas = signal<Problema[]>([]);
  protected readonly errorGeneral = signal<string | null>(null);

  /** Las empresas que coinciden con lo escrito: quién coincide lo decide el núcleo (N2). */
  protected readonly empresasQueCoinciden = computed(() => {
    const empresas = this.empresas();
    const coinciden = this.nucleo.filasQueCoinciden(
      this.empresaTexto(),
      empresas.map((e) => [{ campo: 'EMPRESA' as const, texto: e.nombre }]),
    );
    return empresas.filter((_, i) => coinciden[i]);
  });

  protected mensaje(campo: Campo): string | null {
    return this.problemas().find((p) => p.campo === campo)?.mensaje ?? null;
  }

  protected escribirCedula(evento: Event): void {
    this.cedula.set((evento.target as HTMLInputElement).value);
    this.quitarProblema('cedula');
  }

  protected escribirNombre(evento: Event): void {
    const campo = evento.target as HTMLInputElement;
    const cursor = campo.selectionStart;
    const queda = this.nucleo.nombreAlEscribir(campo.value);
    if (queda !== campo.value) {
      campo.value = queda;
      campo.setSelectionRange(cursor, cursor);
    }
    this.nombre.set(queda);
    this.quitarProblema('nombre');
  }

  protected escribirEmpresa(evento: Event): void {
    const texto = (evento.target as HTMLInputElement).value;
    this.empresaTexto.set(texto);
    // Si cambia lo escrito, la elegida deja de valer hasta que se elija otra.
    if (this.empresaElegida()?.nombre !== texto) {
      this.empresaElegida.set(null);
    }
    this.quitarProblema('empresa_id');
  }

  protected elegirEmpresa(evento: MatAutocompleteSelectedEvent): void {
    const empresa = evento.option.value as EmpresaCatalogo;
    this.empresaElegida.set(empresa);
    this.empresaTexto.set(empresa.nombre);
    this.quitarProblema('empresa_id');
  }

  protected escribirFecha(evento: Event): void {
    this.fecha.set((evento.target as HTMLInputElement).value);
    this.quitarProblema('fecha_vencimiento_praind');
  }

  protected nuevaEmpresa(): void {
    this.dialogos
      .open<DialogoEmpresa, void, EmpresaCatalogo>(DialogoEmpresa, { autoFocus: 'dialog' })
      .afterClosed()
      .subscribe((empresa) => {
        if (empresa) {
          this.empresas.update((todas) =>
            [...todas, empresa].sort((a, b) => a.nombre.localeCompare(b.nombre, 'es')),
          );
          this.empresaElegida.set(empresa);
          this.empresaTexto.set(empresa.nombre);
          this.quitarProblema('empresa_id');
        }
      });
  }

  protected async guardar(evento: Event): Promise<void> {
    evento.preventDefault();
    if (this.enviando()) {
      return;
    }
    this.enviando.set(true);
    this.problemas.set([]);
    this.errorGeneral.set(null);
    const formulario = {
      cedula: this.cedula(),
      nombre: this.nombre(),
      empresa_id: this.empresaElegida()?.id ?? '',
      tipo_ingreso: this.tipo(),
      fecha_vencimiento_praind: this.fecha(),
      tiene_acceso: this.tieneAcceso(),
    };
    try {
      if (this.contratista) {
        const contratista = await this.api.editar(this.contratista.id, formulario);
        this.dialogo.close({ tipo: 'editado', contratista });
      } else {
        const contratista = await this.api.registrar(formulario);
        this.dialogo.close({ tipo: 'registrado', contratista });
      }
    } catch (error) {
      this.mostrar(error);
    } finally {
      this.enviando.set(false);
    }
  }

  protected cancelar(): void {
    this.dialogo.close();
  }

  private quitarProblema(campo: Campo): void {
    this.problemas.update((todos) => todos.filter((p) => p.campo !== campo));
  }

  /** Cada problema junto a su campo; lo que no es de un campo, arriba de los botones. */
  private mostrar(error: unknown): void {
    if (error instanceof ErrorPanel && error.problemas.length > 0) {
      this.problemas.set([...error.problemas]);
      const sueltos = error.problemas.filter((p) => !CAMPOS.some((c) => c === p.campo));
      this.errorGeneral.set(sueltos[0]?.mensaje ?? null);
      return;
    }
    const codigo = error instanceof ErrorPanel ? error.codigo : '';
    this.errorGeneral.set(MENSAJE_DEL_PANEL[codigo] ?? 'No se pudo guardar. Intente de nuevo.');
  }
}
