// El dominio de Rust, compilado a WebAssembly (`dominio/`, hecho por `npm run dominio`),
// cargado una sola vez. Cada módulo del Worker que necesita una regla la importa de aquí.

import { initSync } from './dominio/limen_dominio_wasm.js';
import modulo from './dominio/limen_dominio_wasm_bg.wasm';

initSync({ module: modulo });

export * from './dominio/limen_dominio_wasm.js';
