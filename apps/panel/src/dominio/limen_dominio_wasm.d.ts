/* tslint:disable */
/* eslint-disable */

/**
 * Los datos de un usuario ya en su forma única, tal como el núcleo los decide: la pantalla
 * no normaliza nada, sólo muestra esto.
 */
export class DatosDeUsuario {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cedula: string;
    nombre: string;
}

/**
 * Una clave temporal con el azar que entrega el Worker (`crypto.getRandomValues`, 10 bytes).
 */
export function clave_temporal(azar: Uint8Array, cedula: string): string;

/**
 * El código como se le muestra a la persona (`XXXXX-XXXXX`).
 */
export function codigo_para_mostrar(texto: string): string;

/**
 * Valida y normaliza la cédula y el nombre de un usuario (alta o edición). `cedula_en_uso`:
 * si **otro** usuario ya tiene esa cédula. Si algo no vale, el error es el código de la
 * regla (`cedula_invalida`, `nombre_vacio`, `usuario_cedula_repetida`…).
 */
export function datos_de_usuario(cedula: string, nombre: string, cedula_en_uso: boolean): DatosDeUsuario;

/**
 * Un código nuevo con el azar que entrega el Worker (`crypto.getRandomValues`, 10 bytes).
 */
export function generar_codigo(azar: Uint8Array): string;

/**
 * Lee lo que escribió o escaneó una persona y lo deja en su forma canónica (lo que se
 * transforma en huella).
 */
export function leer_codigo(texto: string): string;

/**
 * ¿Se puede canjear un código emitido en `emitido_ms`? Vacío si sí; si no, el código
 * de la regla que lo impide.
 */
export function motivo_de_rechazo(emitido_ms: number, usado: boolean, anulado: boolean, ahora_ms: number): string;

/**
 * ¿Puede conectarse un equipo con ese estado? Vacío si sí; si no, el código de la regla.
 */
export function motivo_de_rechazo_del_equipo(vinculado: boolean, retirado: boolean): string;

/**
 * El nombre como se ve mientras se escribe (mayúscula, sin tocar los espacios).
 */
export function nombre_al_escribir(texto: string): string;

/**
 * Cuántos minutos vale un código.
 */
export function vigencia_minutos(): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_datosdeusuario_free: (a: number, b: number) => void;
    readonly __wbg_get_datosdeusuario_cedula: (a: number) => [number, number];
    readonly __wbg_get_datosdeusuario_nombre: (a: number) => [number, number];
    readonly __wbg_set_datosdeusuario_cedula: (a: number, b: number, c: number) => void;
    readonly __wbg_set_datosdeusuario_nombre: (a: number, b: number, c: number) => void;
    readonly clave_temporal: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly codigo_para_mostrar: (a: number, b: number) => [number, number, number, number];
    readonly datos_de_usuario: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly generar_codigo: (a: number, b: number) => [number, number, number, number];
    readonly leer_codigo: (a: number, b: number) => [number, number, number, number];
    readonly motivo_de_rechazo: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly motivo_de_rechazo_del_equipo: (a: number, b: number) => [number, number];
    readonly nombre_al_escribir: (a: number, b: number) => [number, number];
    readonly vigencia_minutos: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
