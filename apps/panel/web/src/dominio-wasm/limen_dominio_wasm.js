/* @ts-self-types="./limen_dominio_wasm.d.ts" */

/**
 * Los datos de un usuario ya en su forma única, tal como el núcleo los decide: la pantalla
 * no normaliza nada, sólo muestra esto.
 */
export class DatosDeUsuario {
    static __wrap(ptr) {
        const obj = Object.create(DatosDeUsuario.prototype);
        obj.__wbg_ptr = ptr;
        DatosDeUsuarioFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        DatosDeUsuarioFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datosdeusuario_free(ptr, 0);
    }
    /**
     * @returns {string}
     */
    get cedula() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.__wbg_get_datosdeusuario_cedula(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    get nombre() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.__wbg_get_datosdeusuario_nombre(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} arg0
     */
    set cedula(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_datosdeusuario_cedula(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} arg0
     */
    set nombre(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_datosdeusuario_nombre(this.__wbg_ptr, ptr0, len0);
    }
}
if (Symbol.dispose) DatosDeUsuario.prototype[Symbol.dispose] = DatosDeUsuario.prototype.free;

/**
 * Lo escrito en el buscador de una tabla como se ve (mayúsculas, sin tildes; regla N3).
 * @param {string} texto
 * @returns {string}
 */
export function buscador_al_escribir(texto) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(texto, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.buscador_al_escribir(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * De cada pedido (`{ operacion, celda, filtro }`), si la celda cumple el filtro de columna.
 * @param {string} pedidos
 * @returns {string}
 */
export function celdas_que_cumplen(pedidos) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(pedidos, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.celdas_que_cumplen(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Una clave temporal con el azar que entrega el Worker (`crypto.getRandomValues`, 10 bytes).
 * @param {Uint8Array} azar
 * @param {string} cedula
 * @returns {string}
 */
export function clave_temporal(azar, cedula) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passArray8ToWasm0(azar, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(cedula, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.clave_temporal(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * El código como se le muestra a la persona (`XXXXX-XXXXX`).
 * @param {string} texto
 * @returns {string}
 */
export function codigo_para_mostrar(texto) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(texto, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.codigo_para_mostrar(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * La cédula del contratista en su forma única (vacía si no vale).
 * @param {string} texto
 * @returns {string}
 */
export function contratista_cedula(texto) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(texto, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.contratista_cedula(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Evalúa el formulario de un contratista. Entrada JSON: `{ id, ahora_ms, anterior, borrador,
 * hechos }` (`anterior` es `null` en un alta). Salida JSON: `{ problemas }` con todo lo que
 * impide guardar, o `{ problemas: [], datos, cambios }` con lo que se guarda y se audita.
 * @param {string} entrada
 * @returns {string}
 */
export function contratista_evaluar(entrada) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(entrada, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.contratista_evaluar(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Valida y normaliza la cédula y el nombre de un usuario (alta o edición). `cedula_en_uso`:
 * si **otro** usuario ya tiene esa cédula. Si algo no vale, el error es el código de la
 * regla (`cedula_invalida`, `nombre_vacio`, `usuario_cedula_repetida`…).
 * @param {string} cedula
 * @param {string} nombre
 * @param {boolean} cedula_en_uso
 * @returns {DatosDeUsuario}
 */
export function datos_de_usuario(cedula, nombre, cedula_en_uso) {
    const ptr0 = passStringToWasm0(cedula, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(nombre, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.datos_de_usuario(ptr0, len0, ptr1, len1, cedula_en_uso);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return DatosDeUsuario.__wrap(ret[0]);
}

/**
 * El nombre de empresa como se ve mientras se escribe (mayúscula, sin tocar los espacios).
 * @param {string} texto
 * @returns {string}
 */
export function empresa_al_escribir(texto) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(texto, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.empresa_al_escribir(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Evalúa el nombre de una empresa (alta, o renombrar si `anterior` no está vacío). Devuelve
 * un JSON `{ problema }` o `{ problema: null, nombre, nombre_busqueda, cambios }`.
 * @param {string} id
 * @param {string} anterior
 * @param {string} nombre
 * @param {boolean} nombre_en_uso
 * @returns {string}
 */
export function empresa_evaluar(id, anterior, nombre, nombre_en_uso) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(anterior, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(nombre, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.empresa_evaluar(ptr0, len0, ptr1, len1, ptr2, len2, nombre_en_uso);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * El nombre de empresa en su forma única, vacío si no vale (el nombre se compara así para
 * saber si ya existe).
 * @param {string} texto
 * @returns {string}
 */
export function empresa_nombre(texto) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(texto, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.empresa_nombre(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Cómo está conectado un equipo: `en_linea`, `caido` (sin señal) o `fuera` (desconectado). Lo
 * decide el núcleo; el Worker y la página sólo muestran el código.
 * @param {boolean} anuncia_en_linea
 * @param {number} visto_ms
 * @param {number} ahora_ms
 * @returns {string}
 */
export function estado_de_conexion_del_equipo(anuncia_en_linea, visto_ms, ahora_ms) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.estado_de_conexion_del_equipo(anuncia_en_linea, visto_ms, ahora_ms);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * ¿Es un PRAIND de verdad, o la fecha de quien se registró sin acceso y sin PRAIND?
 * @param {string} fecha
 * @returns {boolean}
 */
export function fecha_es_praind(fecha) {
    const ptr0 = passStringToWasm0(fecha, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.fecha_es_praind(ptr0, len0);
    return ret !== 0;
}

/**
 * De cada fila (un arreglo de `{ campo, texto }`), si coincide con el buscador.
 * @param {string} buscado
 * @param {string} filas
 * @returns {string}
 */
export function filas_que_coinciden(buscado, filas) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(buscado, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(filas, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.filas_que_coinciden(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Un código nuevo con el azar que entrega el Worker (`crypto.getRandomValues`, 10 bytes).
 * @param {Uint8Array} azar
 * @returns {string}
 */
export function generar_codigo(azar) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(azar, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.generar_codigo(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Lee lo que escribió o escaneó una persona y lo deja en su forma canónica (lo que se
 * transforma en huella).
 * @param {string} texto
 * @returns {string}
 */
export function leer_codigo(texto) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(texto, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.leer_codigo(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * El mensaje que el núcleo da a un error de usuario, por su código (vacío si no es de los
 * suyos). La pantalla recibe sólo el código y muestra esto.
 * @param {string} codigo
 * @returns {string}
 */
export function mensaje_de_error_de_usuario(codigo) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(codigo, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.mensaje_de_error_de_usuario(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * ¿Se puede canjear un código emitido en `emitido_ms`? Vacío si sí; si no, el código
 * de la regla que lo impide.
 * @param {number} emitido_ms
 * @param {boolean} usado
 * @param {boolean} anulado
 * @param {number} ahora_ms
 * @returns {string}
 */
export function motivo_de_rechazo(emitido_ms, usado, anulado, ahora_ms) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.motivo_de_rechazo(emitido_ms, usado, anulado, ahora_ms);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * ¿Puede conectarse un equipo con ese estado? Vacío si sí; si no, el código de la regla.
 * @param {boolean} vinculado
 * @param {boolean} retirado
 * @returns {string}
 */
export function motivo_de_rechazo_del_equipo(vinculado, retirado) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.motivo_de_rechazo_del_equipo(vinculado, retirado);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * El nombre como se ve mientras se escribe (mayúscula, sin tocar los espacios).
 * @param {string} texto
 * @returns {string}
 */
export function nombre_al_escribir(texto) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(texto, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.nombre_al_escribir(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * ¿Está vencido el PRAIND que vence en `fecha` (`AAAA-MM-DD`)? Hoy es el de Costa Rica.
 * @param {string} fecha
 * @param {number} ahora_ms
 * @returns {boolean}
 */
export function praind_esta_vencido(fecha, ahora_ms) {
    const ptr0 = passStringToWasm0(fecha, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.praind_esta_vencido(ptr0, len0, ahora_ms);
    return ret !== 0;
}

/**
 * Parte cada texto en segmentos (`[[{ texto, resaltado }]]`). `campo`: `NOMBRE`,
 * `IDENTIFICACION` o `EMPRESA`; `textos`: un arreglo JSON de textos.
 * @param {string} buscado
 * @param {string} campo
 * @param {string} textos
 * @returns {string}
 */
export function resaltar_coincidencias(buscado, campo, textos) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(buscado, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(campo, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(textos, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.resaltar_coincidencias(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * El alta de un usuario. Devuelve un JSON `{ cedula, nombre, cambios }`; si algo no vale, el
 * error es el código de la regla.
 * @param {string} cedula
 * @param {string} nombre
 * @param {boolean} cedula_en_uso
 * @returns {string}
 */
export function usuario_para_alta(cedula, nombre, cedula_en_uso) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(cedula, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(nombre, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.usuario_para_alta(ptr0, len0, ptr1, len1, cedula_en_uso);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * La edición de un usuario. Devuelve un JSON `{ cedula, nombre, activo, cambios }`.
 * @param {string} id
 * @param {string} antes_cedula
 * @param {string} antes_nombre
 * @param {boolean} antes_activo
 * @param {string} cedula
 * @param {string} nombre
 * @param {boolean} activo
 * @param {boolean} cedula_en_uso
 * @returns {string}
 */
export function usuario_para_edicion(id, antes_cedula, antes_nombre, antes_activo, cedula, nombre, activo, cedula_en_uso) {
    let deferred7_0;
    let deferred7_1;
    try {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(antes_cedula, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(antes_nombre, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(cedula, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(nombre, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ret = wasm.usuario_para_edicion(ptr0, len0, ptr1, len1, ptr2, len2, antes_activo, ptr3, len3, ptr4, len4, activo, cedula_en_uso);
        var ptr6 = ret[0];
        var len6 = ret[1];
        if (ret[3]) {
            ptr6 = 0; len6 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred7_0 = ptr6;
        deferred7_1 = len6;
        return getStringFromWasm0(ptr6, len6);
    } finally {
        wasm.__wbindgen_free(deferred7_0, deferred7_1, 1);
    }
}

/**
 * Restablecer la clave de un usuario. Devuelve un JSON `{ cambios }`.
 * @param {string} id
 * @param {string} cedula
 * @param {string} nombre
 * @param {boolean} activo
 * @param {boolean} debe_cambiar_clave
 * @returns {string}
 */
export function usuario_para_restablecer(id, cedula, nombre, activo, debe_cambiar_clave) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(cedula, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(nombre, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.usuario_para_restablecer(ptr0, len0, ptr1, len1, ptr2, len2, activo, debe_cambiar_clave);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * Cuántos minutos vale un código.
 * @returns {number}
 */
export function vigencia_minutos() {
    const ret = wasm.vigencia_minutos();
    return ret;
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_41e9ee4f547fc59a: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./limen_dominio_wasm_bg.js": import0,
    };
}

const DatosDeUsuarioFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_datosdeusuario_free(ptr, 1));

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('limen_dominio_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
