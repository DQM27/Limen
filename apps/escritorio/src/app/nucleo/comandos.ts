import type {
  Cambio,
  CambioGafete,
  ContratistaEntrada,
  Empresa,
  EmpresaProveedora,
  EntradaCorreo,
  EntradaProveedor,
  FilaContratista,
  Gafete,
  PersonalKof,
  TipoGafete,
} from './tipos';
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

/** Cambia el nombre de una empresa; devuelve lo que cambió. */
export function renombrarEmpresa(id: string, nombre: string): Promise<Cambio[]> {
  return invocar<Cambio[]>('renombrar_empresa', { id, nombre });
}

// --- Proveedores ---

export function buscarEmpresasProveedoras(
  texto: string,
  limite: number,
): Promise<EmpresaProveedora[]> {
  return invocar<EmpresaProveedora[]>('buscar_empresas_proveedoras', { texto, limite });
}

export function registrarEmpresaProveedora(nombre: string): Promise<string> {
  return invocar<string>('registrar_empresa_proveedora', { nombre });
}

export function renombrarEmpresaProveedora(id: string, nombre: string): Promise<Cambio[]> {
  return invocar<Cambio[]>('renombrar_empresa_proveedora', { id, nombre });
}

/** Registra la entrada de un proveedor y devuelve el ingreso. */
export function registrarEntradaProveedor(entrada: EntradaProveedor): Promise<string> {
  return invocar<string>('registrar_entrada_proveedor', { entrada });
}

// --- Ingreso por correo ---

/** Registra la entrada de una visita por correo y devuelve el ingreso. */
export function registrarEntradaCorreo(entrada: EntradaCorreo): Promise<string> {
  return invocar<string>('registrar_entrada_correo', { entrada });
}

// --- Personal KOF ---

export function buscarPersonalKof(texto: string, limite: number): Promise<PersonalKof[]> {
  return invocar<PersonalKof[]>('buscar_personal_kof', { texto, limite });
}

export function registrarPersonalKof(codigoEmpleado: string, nombre: string): Promise<string> {
  return invocar<string>('registrar_personal_kof', { codigoEmpleado, nombre });
}

/** Edita el nombre y si está activa (se desactiva, no se borra). */
export function editarPersonalKof(id: string, nombre: string, activo: boolean): Promise<Cambio[]> {
  return invocar<Cambio[]>('editar_personal_kof', { id, nombre, activo });
}

/** Entrega el gafete provisional (es su entrada) y devuelve el préstamo. */
export function entregarGafeteKof(personalId: string, gafete: number): Promise<string> {
  return invocar<string>('entregar_gafete_kof', { personalId, gafete });
}

// --- Gafetes ---

export function listarGafetes(tipo: TipoGafete): Promise<Gafete[]> {
  return invocar<Gafete[]>('listar_gafetes', { tipo });
}

/** Crea los gafetes de un tipo, de `desde` a `hasta`; devuelve cuántos. */
export function registrarGafetes(tipo: TipoGafete, desde: number, hasta: number): Promise<number> {
  return invocar<number>('registrar_gafetes', { tipo, desde, hasta });
}

/** Cambia el estado de un gafete y devuelve lo que cambió. */
export function cambiarGafete(
  tipo: TipoGafete,
  numero: number,
  cambio: CambioGafete,
): Promise<Cambio[]> {
  return invocar<Cambio[]>('cambiar_gafete', { tipo, numero, cambio });
}
