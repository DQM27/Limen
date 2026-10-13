import type {
  AtajoFecha,
  Cambio,
  CambioGafete,
  CampoDeTexto,
  CampoResaltable,
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
  Historial,
  PedidoDeFiltro,
  PersonaAdentro,
  PersonalKof,
  Segmento,
  TipoGafete,
  UsuarioActual,
  Via,
  EstadoVinculacion,
} from './tipos';
import { invocar, type ErrorApp } from './tauri';

/**
 * Los comandos del núcleo con su forma tipada: un método por comando de
 * Tauri, sin lógica. Es el único lugar que conoce los nombres de los
 * comandos y de sus parámetros.
 */

export function listarContratistas(): Promise<FilaContratista[]> {
  return invocar<FilaContratista[]>('listar_contratistas');
}

/**
 * Cada texto partido en segmentos, con lo que coincide con lo buscado
 * resaltado (regla N1). Devuelve un arreglo de segmentos por cada texto, en el
 * mismo orden. No toca datos: sólo aplica la regla del núcleo.
 */
export function resaltarCoincidencias(
  buscado: string,
  campo: CampoResaltable,
  textos: string[],
): Promise<Segmento[][]> {
  return invocar<Segmento[][]>('resaltar_coincidencias', { buscado, campo, textos });
}

/**
 * De cada fila de una tabla, si coincide con lo escrito en el buscador
 * (regla N2). Una respuesta por fila, en el mismo orden.
 */
export function filasQueCoinciden(buscado: string, filas: CampoDeTexto[][]): Promise<boolean[]> {
  return invocar<boolean[]>('filas_que_coinciden', { buscado, filas });
}

/**
 * De cada celda, si cumple el filtro de columna que se le pide (regla N2).
 * Una respuesta por pedido, en el mismo orden.
 */
export function celdasQueCumplen(pedidos: PedidoDeFiltro[]): Promise<boolean[]> {
  return invocar<boolean[]>('celdas_que_cumplen', { pedidos });
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
 * Todo lo que impide guardar el formulario, de una vez y cada problema con su
 * `campo` (lista vacía si se puede guardar). `id` es el contratista que se
 * edita, o `null` si es uno nuevo. No guarda nada.
 */
export function validarContratista(
  id: string | null,
  contratista: ContratistaEntrada,
): Promise<ErrorApp[]> {
  return invocar<ErrorApp[]>('validar_contratista', { id, contratista });
}

/**
 * El nombre de una persona como quedará al guardarse (mayúsculas, sin
 * tildes), para mostrarlo mientras se escribe. Lo decide el dominio.
 */
export function nombrePersonaAlEscribir(texto: string): Promise<string> {
  return invocar<string>('nombre_persona_al_escribir', { texto });
}

/** La placa de un vehículo como quedará al guardarse (mayúsculas). */
export function placaAlEscribir(texto: string): Promise<string> {
  return invocar<string>('placa_al_escribir', { texto });
}

/** Lo mismo para el nombre de una empresa (mayúsculas). */
export function nombreEmpresaAlEscribir(texto: string): Promise<string> {
  return invocar<string>('nombre_empresa_al_escribir', { texto });
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

/**
 * Todo lo que impide registrar el ingreso de un proveedor, de una vez y cada
 * problema con su `campo` (lista vacía si se puede). No registra nada.
 */
export function validarEntradaProveedor(entrada: EntradaProveedor): Promise<ErrorApp[]> {
  return invocar<ErrorApp[]>('validar_entrada_proveedor', { entrada });
}

/** Registra la entrada de un proveedor y devuelve el ingreso. */
export function registrarEntradaProveedor(entrada: EntradaProveedor): Promise<string> {
  return invocar<string>('registrar_entrada_proveedor', { entrada });
}

// --- Ingreso por correo ---

/**
 * Todo lo que impide registrar el ingreso por correo, de una vez y cada
 * problema con su `campo` (lista vacía si se puede). No registra nada.
 */
export function validarEntradaCorreo(entrada: EntradaCorreo): Promise<ErrorApp[]> {
  return invocar<ErrorApp[]>('validar_entrada_correo', { entrada });
}

/** Registra la entrada de una visita por correo y devuelve el ingreso. */
export function registrarEntradaCorreo(entrada: EntradaCorreo): Promise<string> {
  return invocar<string>('registrar_entrada_correo', { entrada });
}

// --- Personal KOF ---

export function buscarPersonalKof(texto: string, limite: number): Promise<PersonalKof[]> {
  return invocar<PersonalKof[]>('buscar_personal_kof', { texto, limite });
}

/**
 * Todo lo que impide dar de alta a esta persona, de una vez y cada problema
 * con su `campo` (lista vacía si se puede). No registra nada.
 */
export function validarPersonalKof(codigoEmpleado: string, nombre: string): Promise<ErrorApp[]> {
  return invocar<ErrorApp[]>('validar_personal_kof', { codigoEmpleado, nombre });
}

export function registrarPersonalKof(codigoEmpleado: string, nombre: string): Promise<string> {
  return invocar<string>('registrar_personal_kof', { codigoEmpleado, nombre });
}

/** Edita el nombre y si está activa (se desactiva, no se borra). */
export function editarPersonalKof(id: string, nombre: string, activo: boolean): Promise<Cambio[]> {
  return invocar<Cambio[]>('editar_personal_kof', { id, nombre, activo });
}

/**
 * Todo lo que impide entregar el gafete provisional, de una vez y cada
 * problema con su `campo` (lista vacía si se puede). `personalId` vacío es
 * «todavía no eligió a la persona». No entrega nada.
 */
export function validarEntregaGafeteKof(personalId: string, gafete: number): Promise<ErrorApp[]> {
  return invocar<ErrorApp[]>('validar_entrega_gafete_kof', { personalId, gafete });
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

/**
 * Registra la salida de quien devuelve el gafete con este número y devuelve
 * su nombre. Sin vía (`null`) lo busca en las cuatro.
 */
export function registrarSalidaPorGafete(via: Via | null, numero: number): Promise<string> {
  return invocar<string>('registrar_salida_por_gafete', { via, numero });
}

// --- Sesión (bloque L) ---
// Los usuarios vienen de la nube: el equipo no los crea ni los administra.

/**
 * Entra con cédula y clave. Si falla, el error nunca dice cuál de las
 * dos estaba mal (`credenciales_invalidas`), o que hay que esperar
 * (`inicio_bloqueado`).
 */
export function iniciarSesion(cedula: string, clave: string): Promise<UsuarioActual> {
  return invocar<UsuarioActual>('iniciar_sesion', { cedula, clave });
}

export function cerrarSesion(): Promise<void> {
  return invocar<void>('cerrar_sesion');
}

/** Quién tiene la sesión, o `null` si nadie. */
export function usuarioActual(): Promise<UsuarioActual | null> {
  return invocar<UsuarioActual | null>('usuario_actual');
}

/** Cambia la clave propia: lo único permitido con una temporal. */
export function cambiarClave(claveActual: string, clave: string): Promise<void> {
  return invocar<void>('cambiar_clave', { claveActual, clave });
}

// --- Historial de ingresos ---

/**
 * Los ingresos y salidas de las cuatro vías entre `desde` y `hasta`
 * (`AAAA-MM-DD`, días de Costa Rica, ambos incluidos; `null` = sin límite).
 * Un rango invertido rechaza con `campo: 'hasta'`; una fecha ilegible, con su
 * campo.
 */
export function listarHistorial(desde: string | null, hasta: string | null): Promise<Historial> {
  return invocar<Historial>('listar_historial', { desde, hasta });
}

/** Los accesos rápidos de fecha, con el rango que da cada uno hoy. */
export function atajosDeFecha(): Promise<AtajoFecha[]> {
  return invocar<AtajoFecha[]>('atajos_de_fecha');
}

// --- Vinculación del equipo (bloque M) ---

/** Si el equipo ya se vinculó con un código. Se pregunta antes de iniciar sesión. */
export function estadoVinculacion(): Promise<EstadoVinculacion> {
  return invocar<EstadoVinculacion>('estado_vinculacion_equipo');
}

/**
 * Canjea el código del administrador y deja guardada la identidad del equipo. Los
 * errores traen `campo: 'codigo'` (código mal escrito, vencido, usado…) o son generales
 * (sin conexión con el panel).
 */
export function vincularEquipo(codigo: string): Promise<void> {
  return invocar<void>('vincular_equipo_con_codigo', { codigo });
}

/** Reinicia la app: tras vincular, la base se abre con la identidad nueva. */
export function reiniciar(): Promise<void> {
  return invocar<void>('reiniciar');
}
