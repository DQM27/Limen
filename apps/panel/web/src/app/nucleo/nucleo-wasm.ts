import { Injectable } from '@angular/core';
import init, {
  buscador_al_escribir,
  celdas_que_cumplen,
  datos_de_usuario,
  empresa_al_escribir,
  estado_de_conexion_del_equipo,
  fecha_es_praind,
  filas_que_coinciden,
  mensaje_de_error_de_usuario,
  nombre_al_escribir,
  praind_esta_vencido,
  resaltar_coincidencias,
} from '../../dominio-wasm/limen_dominio_wasm.js';
import rutaDelWasm from '../../dominio-wasm/limen_dominio_wasm_bg.wasm';
import {
  Nucleo,
  type CampoDeTexto,
  type CampoResaltable,
  type EstadoDeConexion,
  type OperacionDeFiltro,
  type ResultadoDeDatos,
  type Segmento,
} from './nucleo';

/** Carga el WebAssembly del núcleo. Hay que esperarla antes de mostrar nada (`app.config.ts`). */
export async function iniciarNucleo(): Promise<void> {
  await init({ module_or_path: rutaDelWasm });
}

/** El núcleo de verdad: el dominio de Rust compilado a WebAssembly (`npm run dominio`). */
@Injectable()
export class NucleoWasm extends Nucleo {
  celdaCumple(operacion: OperacionDeFiltro, celda: string, filtro: string): boolean {
    const respuesta = JSON.parse(
      celdas_que_cumplen(JSON.stringify([{ operacion, celda, filtro }])),
    ) as boolean[];
    return respuesta[0] ?? true;
  }

  filasQueCoinciden(buscado: string, filas: CampoDeTexto[][]): boolean[] {
    return JSON.parse(filas_que_coinciden(buscado, JSON.stringify(filas))) as boolean[];
  }

  resaltar(buscado: string, campo: CampoResaltable, textos: string[]): Segmento[][] {
    return JSON.parse(
      resaltar_coincidencias(buscado, campo, JSON.stringify(textos)),
    ) as Segmento[][];
  }

  buscadorAlEscribir(texto: string): string {
    return buscador_al_escribir(texto);
  }

  nombreAlEscribir(texto: string): string {
    return nombre_al_escribir(texto);
  }

  datosDeUsuario(cedula: string, nombre: string): ResultadoDeDatos {
    try {
      const datos = datos_de_usuario(cedula, nombre, false);
      return { ok: true, datos: { cedula: datos.cedula, nombre: datos.nombre } };
    } catch (error) {
      // El núcleo avisa con el código de la regla como texto.
      const codigo = typeof error === 'string' ? error : 'desconocido';
      return { ok: false, codigo, mensaje: this.mensajeDeError(codigo) };
    }
  }

  estadoDeConexion(enLinea: boolean, vistoMs: number, ahoraMs: number): EstadoDeConexion {
    return estado_de_conexion_del_equipo(enLinea, vistoMs, ahoraMs) as EstadoDeConexion;
  }

  mensajeDeError(codigo: string): string {
    return mensaje_de_error_de_usuario(codigo);
  }

  empresaAlEscribir(texto: string): string {
    return empresa_al_escribir(texto);
  }

  fechaEsPraind(fecha: string): boolean {
    return fecha_es_praind(fecha);
  }

  praindVencido(fecha: string, ahoraMs: number): boolean {
    return praind_esta_vencido(fecha, ahoraMs);
  }
}
