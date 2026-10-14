// `node --import ./apps/panel/test/wasm.mjs --test apps/panel/test`: registra el cargador de `.wasm`.
import { register } from 'node:module';

register('./wasm-cargador.mjs', import.meta.url);
