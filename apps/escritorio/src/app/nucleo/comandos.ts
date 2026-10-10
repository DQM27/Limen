import type {
  Cambio,
  CambioGafete,
  CandidatoIngreso,
  ContratistaEntrada,
  Empresa,
  EmpresaProveedora,
  EntradaContratista,
  EntradaCorreo,
  EntradaRegistrada,
  EntradaProveedor,
  FilaContratista,
  Gafete,
  PersonaAdentro,
  PersonalKof,
  TipoGafete,
  Usuario,
  UsuarioActual,
  UsuarioEntrada,
  Via,
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

// --- Ingreso de contratista ---

/** El buscador del ingreso: cédula o nombre en el mismo campo. */
export function buscarParaIngreso(texto: string, limite: number): Promise<CandidatoIngreso[]> {
  return invocar<CandidatoIngreso[]>('buscar_para_ingreso', { texto, limite });
}

/** La ficha del contratista elegido, antes de registrar su entrada. */
export function prepararIngreso(contratistaId: string): Promise<CandidatoIngreso> {
  return invocar<CandidatoIngreso>('preparar_ingreso', { contratistaId });
}

/** Registra la entrada; si algo no pasa, el error trae su `campo`. */
export function registrarEntradaContratista(
  entrada: EntradaContratista,
): Promise<EntradaRegistrada> {
  return invocar<EntradaRegistrada>('registrar_entrada_contratista', { entrada });
}

// --- Dentro y salidas ---

/** Quién está adentro ahora, por las cuatro vías. */
export function dentro(): Promise<PersonaAdentro[]> {
  return invocar<PersonaAdentro[]>('dentro');
}

/** Registra la salida desde la lista «dentro». */
export function registrarSalida(via: Via, ingresoId: string): Promise<void> {
  return invocar<void>('registrar_salida', { via, ingresoId });
}

/** Registra la salida por el número del gafete que devuelve la persona. */
export function registrarSalidaPorGafete(via: Via, numero: number): Promise<void> {
  return invocar<void>('registrar_salida_por_gafete', { via, numero });
}

// --- Sesión (bloque L) ---

/** Si el equipo ya tiene usuarios; sin ninguno se ofrece crear el primero. */
export function hayUsuarios(): Promise<boolean> {
  return invocar<boolean>('hay_usuarios');
}

/** Crea el primer usuario del equipo (sólo si no hay ninguno) y abre su sesión. */
export function crearPrimerUsuario(usuario: UsuarioEntrada): Promise<UsuarioActual> {
  return invocar<UsuarioActual>('crear_primer_usuario', { usuario });
}

/**
 * Entra con cédula y contraseña. Si falla, el error nunca dice cuál de las
 * dos estaba mal (`credenciales_invalidas`), o que hay que esperar
 * (`inicio_bloqueado`).
 */
export function iniciarSesion(cedula: string, contrasena: string): Promise<UsuarioActual> {
  return invocar<UsuarioActual>('iniciar_sesion', { cedula, contrasena });
}

export function cerrarSesion(): Promise<void> {
  return invocar<void>('cerrar_sesion');
}

/** Quién tiene la sesión, o `null` si nadie. */
export function usuarioActual(): Promise<UsuarioActual | null> {
  return invocar<UsuarioActual | null>('usuario_actual');
}

/** Cambia la contraseña propia: lo único permitido con una temporal. */
export function cambiarContrasena(contrasenaActual: string, contrasena: string): Promise<void> {
  return invocar<void>('cambiar_contrasena', { contrasenaActual, contrasena });
}

// --- Usuarios (se administran en el equipo hasta que exista el panel de la nube) ---

export function listarUsuarios(): Promise<Usuario[]> {
  return invocar<Usuario[]>('listar_usuarios');
}

/** Registra un usuario con una contraseña temporal y devuelve su identificador. */
export function registrarUsuario(usuario: UsuarioEntrada): Promise<string> {
  return invocar<string>('registrar_usuario', { usuario });
}

/** Edita el nombre y si está activo (se desactiva, no se borra). */
export function editarUsuario(id: string, nombre: string, activo: boolean): Promise<Cambio[]> {
  return invocar<Cambio[]>('editar_usuario', { id, nombre, activo });
}

/** Le pone una contraseña temporal a otro usuario (por ejemplo, si la olvidó). */
export function restablecerContrasena(id: string, contrasena: string): Promise<void> {
  return invocar<void>('restablecer_contrasena', { id, contrasena });
}
