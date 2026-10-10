import type { Cambio, ContratistaEntrada, Empresa, FilaContratista } from './tipos';
import { invocar } from './tauri';

/**
 * Los comandos del núcleo con su forma tipada: un método por comando de
 * Tauri, sin lógica. Es el único lugar que conoce los nombres de los
 * comandos y de sus parámetros.
 */

export function listarContratistas(): Promise<FilaContratista[]> {
  return invocar<FilaContratista[]>('listar_contratistas');
}

/** Empresas de contratistas cuyo nombre coincide, de la mejor a la peor. */
export function buscarEmpresas(texto: string, limite: number): Promise<Empresa[]> {
  return invocar<Empresa[]>('buscar_empresas', { texto, limite });
}

/** Registra una empresa y devuelve su identificador. */
export function registrarEmpresa(nombre: string): Promise<string> {
  return invocar<string>('registrar_empresa', { nombre });
}

/**
 * Registra un contratista y devuelve su identificador. Si un dato no pasa,
 * rechaza con un `ErrorApp` que trae el `campo` del formulario.
 */
export function registrarContratista(contratista: ContratistaEntrada): Promise<string> {
  return invocar<string>('registrar_contratista', { contratista });
}

/**
 * Edita un contratista con el formulario completo y devuelve lo que cambió
 * (vacío si nada cambió). Los errores traen su `campo`, como al registrar.
 */
export function editarContratista(id: string, contratista: ContratistaEntrada): Promise<Cambio[]> {
  return invocar<Cambio[]>('editar_contratista', { id, contratista });
}
