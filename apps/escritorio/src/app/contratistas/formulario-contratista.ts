import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  Injector,
  signal,
} from '@angular/core';
import { FormControl, NonNullableFormBuilder, ReactiveFormsModule } from '@angular/forms';
import { MatAutocompleteModule } from '@angular/material/autocomplete';
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
import { MatTooltipModule } from '@angular/material/tooltip';
import { DialogoEmpresa } from '../compartido/dialogo-empresa';
import { NormalizadorAlEscribir } from '../compartido/escribir-normalizado';
import {
  buscarEmpresas,
  editarContratista,
  nombreEmpresaAlEscribir,
  nombrePersonaAlEscribir,
  registrarContratista,
  validarContratista,
} from '../nucleo/comandos';
import { esErrorApp, type ErrorApp } from '../nucleo/tauri';
import type { Cambio, ContratistaEntrada, Empresa, FilaContratista } from '../nucleo/tipos';
import { etiquetaDeTipo } from './etiquetas';

/** Con `contratista` el formulario edita a esa persona; sin él, registra una nueva. */
export interface DatosFormularioContratista {
  contratista?: FilaContratista;
}

/** Lo que devuelve el diálogo al cerrarse con éxito. `cambios` vacío = nada cambió. */
export type ResultadoFormularioContratista =
  { tipo: 'registrado' } | { tipo: 'editado'; cambios: Cambio[] };

/** Cuántas empresas se ofrecen al escribir. */
const EMPRESAS_OFRECIDAS = 8;

/** El campo del núcleo (`ErrorApp.campo`) → el control del formulario que lo muestra. */
const CONTROL_DEL_CAMPO: Record<string, 'cedula' | 'nombre' | 'empresa' | 'tipo' | 'fecha'> = {
  cedula: 'cedula',
  nombre: 'nombre',
  empresa_id: 'empresa',
  tipo_ingreso: 'tipo',
  fecha_vencimiento_praind: 'fecha',
};

/**
 * El formulario del contratista, en un diálogo: sirve para registrar y para
 * editar (cambia el título y lo que se precarga). No decide nada: el núcleo
 * normaliza y valida, y cada error trae el campo donde mostrarlo.
 *
 * La empresa sólo se elige entre las que existen; crear una nueva es un paso
 * aparte (el botón "+", que abre su propio modal) para no crearlas por error.
 */
@Component({
  selector: 'app-formulario-contratista',
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
    MatTooltipModule,
    ReactiveFormsModule,
  ],
  templateUrl: './formulario-contratista.html',
  styleUrl: './formulario-contratista.scss',
})
export class FormularioContratista {
  private readonly dialogo =
    inject<MatDialogRef<FormularioContratista, ResultadoFormularioContratista>>(MatDialogRef);
  private readonly dialogos = inject(MatDialog);
  private readonly datos = inject<DatosFormularioContratista | null>(MAT_DIALOG_DATA, {
    optional: true,
  });
  private readonly anfitrion = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly injector = inject(Injector);
  private readonly normalizaNombre = new NormalizadorAlEscribir();
  private readonly normalizaEmpresa = new NormalizadorAlEscribir();

  protected readonly editando = this.datos?.contratista !== undefined;
  protected readonly etiquetaDeTipo = etiquetaDeTipo;

  /** `empresa` es el texto que se escribe para buscar, o la empresa ya elegida de la lista. */
  readonly form = inject(NonNullableFormBuilder).group({
    cedula: '',
    nombre: '',
    empresa: '' as string | Empresa,
    tipo: 'PRAIND' as ContratistaEntrada['tipo_ingreso'],
    fecha: '',
    tieneAcceso: true,
  });

  protected readonly opciones = signal<Empresa[]>([]);
  /** Se escribió algo y ninguna empresa coincide: se avisa dónde crear una. */
  protected readonly sinCoincidencias = signal(false);
  protected readonly enviando = signal(false);
  protected readonly errorGeneral = signal<string | null>(null);

  constructor() {
    const actual = this.datos?.contratista;
    if (actual) {
      this.form.setValue({
        cedula: actual.cedula,
        nombre: actual.nombre,
        empresa: { id: actual.empresa_id, nombre: actual.empresa_nombre },
        tipo: actual.tipo_ingreso,
        fecha: actual.fecha_vencimiento_praind,
        tieneAcceso: actual.tiene_acceso,
      });
    }
    // Un clic fuera del diálogo no lo cierra (no se pierde lo escrito por
    // accidente); Escape sí, como cualquier diálogo.
    this.dialogo.keydownEvents().subscribe((evento) => {
      if (evento.key === 'Escape') {
        this.cancelar();
      }
    });
    this.form.controls.empresa.valueChanges.subscribe((valor) => void this.ofrecerEmpresas(valor));
  }

  protected nombreDeEmpresa = (valor: string | Empresa | null): string =>
    typeof valor === 'string' ? valor : (valor?.nombre ?? '');

  /** Al escribir, la lista trae las empresas que se parecen (el núcleo decide cuáles). */
  private async ofrecerEmpresas(valor: string | Empresa): Promise<void> {
    if (typeof valor !== 'string') {
      return;
    }
    const texto = valor.trim();
    if (texto === '') {
      this.opciones.set([]);
      this.sinCoincidencias.set(false);
      return;
    }
    const encontradas = await buscarEmpresas(texto, EMPRESAS_OFRECIDAS).catch(() => []);
    this.opciones.set(encontradas);
    this.sinCoincidencias.set(encontradas.length === 0);
  }

  protected escribirNombre(evento: Event): void {
    void this.normalizaNombre.alEscribir(
      evento,
      this.form.controls.nombre,
      nombrePersonaAlEscribir,
    );
  }

  /** La empresa se escribe como texto (para buscar) hasta que se elige una de la lista. */
  protected escribirEmpresa(evento: Event): void {
    const control = this.form.controls.empresa as unknown as FormControl<string>;
    void this.normalizaEmpresa.alEscribir(evento, control, nombreEmpresaAlEscribir);
  }

  protected alElegirEmpresa(elegida: Empresa): void {
    this.form.controls.empresa.setValue(elegida, { emitEvent: false });
  }

  /** El botón "+": abre el modal de empresa nueva y, si se creó, la deja elegida. */
  protected crearEmpresa(): void {
    this.dialogos
      .open<DialogoEmpresa, void, Empresa>(DialogoEmpresa, {
        width: '340px',
        disableClose: true,
        autoFocus: 'first-tabbable',
      })
      .afterClosed()
      .subscribe((creada) => {
        if (creada) {
          this.form.controls.empresa.setValue(creada, { emitEvent: false });
          this.sinCoincidencias.set(false);
        }
      });
  }

  protected async guardar(): Promise<void> {
    if (this.enviando()) {
      return;
    }
    this.errorGeneral.set(null);
    this.enviando.set(true);
    try {
      const entrada = this.entrada();
      const actual = this.datos?.contratista;
      // Primero se pregunta qué falta o qué está mal, todo de una vez: cada
      // problema se marca en su campo (lo decide el dominio).
      const problemas = await validarContratista(actual?.id ?? null, entrada);
      if (problemas.length > 0) {
        this.mostrarTodos(problemas);
        return;
      }
      if (actual) {
        const cambios = await editarContratista(actual.id, entrada);
        this.dialogo.close({ tipo: 'editado', cambios });
      } else {
        await registrarContratista(entrada);
        this.dialogo.close({ tipo: 'registrado' });
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

  /**
   * Lo escrito, tal cual: el núcleo normaliza y valida. Si la empresa se
   * escribió completa pero no se eligió, se toma la de la lista que se llama
   * igual; si no hay ninguna, va vacía y el núcleo dice que falta.
   */
  private entrada(): ContratistaEntrada {
    const { cedula, nombre, empresa, tipo, fecha, tieneAcceso } = this.form.getRawValue();
    const escrita = typeof empresa === 'string' ? empresa.trim().toLowerCase() : null;
    const elegida =
      typeof empresa === 'string'
        ? this.opciones().find((e) => escrita !== '' && e.nombre.toLowerCase() === escrita)
        : empresa;
    return {
      cedula,
      nombre,
      empresa_id: elegida?.id ?? '',
      tipo_ingreso: tipo,
      fecha_vencimiento_praind: fecha,
      tiene_acceso: tieneAcceso,
    };
  }

  /** Cada problema junto a su campo (el primero de cada uno); el cursor va al primer campo con problema. */
  private mostrarTodos(problemas: ErrorApp[]): void {
    for (const problema of problemas) {
      const nombre = CONTROL_DEL_CAMPO[problema.campo ?? ''];
      const control = nombre ? this.form.controls[nombre] : null;
      if (control && !control.errors) {
        control.setErrors({ nucleo: problema.mensaje });
        control.markAsTouched();
      } else if (!control) {
        this.errorGeneral.set(problema.mensaje);
      }
    }
    afterNextRender(
      () =>
        this.anfitrion.nativeElement
          .querySelector<HTMLElement>(
            '.mat-form-field-invalid input, .mat-form-field-invalid mat-select',
          )
          ?.focus(),
      { injector: this.injector },
    );
  }

  /** El mensaje del núcleo junto a su campo; sin campo, arriba de los botones. */
  private mostrar(error: unknown): void {
    if (!esErrorApp(error)) {
      this.errorGeneral.set('No se pudo guardar.');
      return;
    }
    const nombre = CONTROL_DEL_CAMPO[error.campo ?? ''];
    if (nombre) {
      const control = this.form.controls[nombre];
      control.setErrors({ nucleo: error.mensaje });
      control.markAsTouched();
    } else {
      this.errorGeneral.set(error.mensaje);
    }
  }
}
