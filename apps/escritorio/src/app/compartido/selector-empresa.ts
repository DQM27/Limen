import { ChangeDetectionStrategy, Component, effect, inject, input, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatAutocompleteModule } from '@angular/material/autocomplete';
import { MatButtonModule } from '@angular/material/button';
import { MatDialog } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatTooltipModule } from '@angular/material/tooltip';
import {
  buscarEmpresas,
  buscarEmpresasProveedoras,
  nombreEmpresaAlEscribir,
} from '../nucleo/comandos';
import type { Empresa } from '../nucleo/tipos';
import {
  DialogoEmpresa,
  type CatalogoDeEmpresas,
  type DatosDialogoEmpresa,
} from './dialogo-empresa';
import { NormalizadorAlEscribir } from './escribir-normalizado';

/** Cuántas empresas se ofrecen al escribir. */
const EMPRESAS_OFRECIDAS = 8;

/**
 * El selector de empresa de un formulario: se escribe para buscar y se elige
 * de la lista; sólo elige entre las que existen. Crear una nueva es un paso
 * aparte (el botón «+», que abre su propio modal) para no crearlas por error.
 * El control es del formulario, que también pone ahí los errores del núcleo.
 */
@Component({
  selector: 'app-selector-empresa',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    MatAutocompleteModule,
    MatButtonModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    MatTooltipModule,
    ReactiveFormsModule,
  ],
  templateUrl: './selector-empresa.html',
  styleUrl: './selector-empresa.scss',
})
export class SelectorEmpresa {
  /** El texto que se escribe para buscar, o la empresa ya elegida de la lista. */
  readonly control = input.required<FormControl<string | Empresa>>();
  readonly catalogo = input<CatalogoDeEmpresas>('contratistas');
  readonly etiqueta = input('Empresa');

  private readonly dialogos = inject(MatDialog);
  private readonly normaliza = new NormalizadorAlEscribir();

  protected readonly opciones = signal<Empresa[]>([]);
  /** Se escribió algo y ninguna empresa coincide: se avisa dónde crear una. */
  protected readonly sinCoincidencias = signal(false);

  constructor() {
    effect((limpiar) => {
      const suscripcion = this.control().valueChanges.subscribe(
        (valor) => void this.ofrecer(valor),
      );
      limpiar(() => suscripcion.unsubscribe());
    });
  }

  protected nombreDeEmpresa = (valor: string | Empresa | null): string =>
    typeof valor === 'string' ? valor : (valor?.nombre ?? '');

  /**
   * El id de la empresa elegida. Si se escribió el nombre completo pero no se
   * eligió, se toma la de la lista que se llama igual; si no hay ninguna, va
   * vacío y el núcleo dice que falta.
   */
  idElegido(): string {
    const valor = this.control().value;
    if (typeof valor !== 'string') {
      return valor.id;
    }
    const escrita = valor.trim().toLowerCase();
    return (
      this.opciones().find((e) => escrita !== '' && e.nombre.toLowerCase() === escrita)?.id ?? ''
    );
  }

  /** Al escribir, la lista trae las empresas que se parecen (el núcleo decide cuáles). */
  private async ofrecer(valor: string | Empresa): Promise<void> {
    if (typeof valor !== 'string') {
      return;
    }
    const texto = valor.trim();
    if (texto === '') {
      this.opciones.set([]);
      this.sinCoincidencias.set(false);
      return;
    }
    const buscar = this.catalogo() === 'proveedoras' ? buscarEmpresasProveedoras : buscarEmpresas;
    const encontradas = await buscar(texto, EMPRESAS_OFRECIDAS).catch(() => []);
    this.opciones.set(encontradas);
    this.sinCoincidencias.set(encontradas.length === 0);
  }

  /** Mientras se escribe, el nombre se ve en mayúsculas (la forma la da el dominio). */
  protected escribir(evento: Event): void {
    const control = this.control() as unknown as FormControl<string>;
    void this.normaliza.alEscribir(evento, control, nombreEmpresaAlEscribir);
  }

  protected alElegir(elegida: Empresa): void {
    this.control().setValue(elegida, { emitEvent: false });
  }

  /** El botón «+»: abre el modal de empresa nueva y, si se creó, la deja elegida. */
  protected crear(): void {
    this.dialogos
      .open<DialogoEmpresa, DatosDialogoEmpresa, Empresa>(DialogoEmpresa, {
        data: { catalogo: this.catalogo() },
        width: '340px',
        disableClose: true,
        autoFocus: 'first-tabbable',
      })
      .afterClosed()
      .subscribe((creada) => {
        if (creada) {
          this.control().setValue(creada, { emitEvent: false });
          this.sinCoincidencias.set(false);
        }
      });
  }
}
