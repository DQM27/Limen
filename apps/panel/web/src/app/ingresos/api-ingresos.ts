import { Injectable } from '@angular/core';
import {
  listarAtajosDeFecha,
  listarIngresos,
  listarUnidades,
  type AtajoDeFecha,
  type FiltroDeIngresos,
  type ListaDeIngresos,
  type Unidad,
} from '../panel';

/**
 * Lo que la pantalla de ingresos le pide al Worker. Es un servicio para que las pruebas lo
 * reemplacen por uno falso; no decide nada.
 */
@Injectable({ providedIn: 'root' })
export class ApiIngresos {
  listar(filtro: FiltroDeIngresos): Promise<ListaDeIngresos> {
    return listarIngresos(filtro);
  }

  atajos(): Promise<{ por_omision: string; atajos: AtajoDeFecha[] }> {
    return listarAtajosDeFecha();
  }

  unidades(): Promise<Unidad[]> {
    return listarUnidades();
  }
}
