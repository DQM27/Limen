/**
 * Lo que la página le pregunta al núcleo (el dominio de Rust, compilado a WebAssembly). La
 * página no compara, no normaliza ni valida: pregunta y muestra la respuesta. Las pantallas
 * y las pruebas dependen de esta clase abstracta; la de verdad (`NucleoWasm`) sólo la usa
 * `app.config.ts`, así las pruebas corren sin cargar el WebAssembly.
 */

/** Qué clase de texto es (decide con qué regla se compara): regla N1 y N2. */
export type CampoResaltable = 'NOMBRE' | 'IDENTIFICACION' | 'EMPRESA';

/** Un texto de una fila de la tabla y de qué clase es. */
export interface CampoDeTexto {
  campo: CampoResaltable;
  texto: string;
}

/** Un tramo de un texto; los de un texto, en orden, lo reconstruyen exacto. */
export interface Segmento {
  texto: string;
  resaltado: boolean;
}

/** Cómo quedan la cédula y el nombre de un usuario según el núcleo. */
export interface DatosDeUsuario {
  cedula: string;
  nombre: string;
}

export type ResultadoDeDatos =
  { ok: true; datos: DatosDeUsuario } | { ok: false; codigo: string; mensaje: string };

/** Cómo está conectado un equipo: lo decide el núcleo (regla de los diez minutos sin avisar). */
export type EstadoDeConexion = 'en_linea' | 'caido' | 'fuera';

export abstract class Nucleo {
  /** De cada fila, si coincide con lo escrito en el buscador de la tabla (regla N2). */
  abstract filasQueCoinciden(buscado: string, filas: CampoDeTexto[][]): boolean[];

  /** Cada texto partido en segmentos, con lo que coincide resaltado (regla N1). */
  abstract resaltar(buscado: string, campo: CampoResaltable, textos: string[]): Segmento[][];

  /** El nombre como se ve mientras se escribe (mayúscula, sin tocar los espacios). */
  abstract nombreAlEscribir(texto: string): string;

  /** Cómo quedarán la cédula y el nombre, o por qué no valen. La cédula única la decide la nube. */
  abstract datosDeUsuario(cedula: string, nombre: string): ResultadoDeDatos;

  /** Cómo está conectado un equipo, con la hora de su última señal y la de ahora (en milisegundos). */
  abstract estadoDeConexion(enLinea: boolean, vistoMs: number, ahoraMs: number): EstadoDeConexion;

  /** El mensaje del núcleo para el código de un error de usuario (vacío si no es suyo). */
  abstract mensajeDeError(codigo: string): string;

  /** El nombre de una empresa como se ve mientras se escribe (mayúscula, sin tocar los espacios). */
  abstract empresaAlEscribir(texto: string): string;

  /** ¿Está vencido el PRAIND que vence en `fecha` (`AAAA-MM-DD`)? `ahoraMs`: la hora de ahora. */
  abstract praindVencido(fecha: string, ahoraMs: number): boolean;
}
