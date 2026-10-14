// Enseña a Node a importar un `.wasm` como lo hace el empaquetador de Cloudflare: el módulo ya
// compilado (`WebAssembly.Module`), listo para `initSync`. Sólo para las pruebas.
import { readFile } from 'node:fs/promises';

export async function load(url, contexto, siguiente) {
  if (url.endsWith('.wasm')) {
    const bytes = await readFile(new URL(url));
    return {
      format: 'module',
      shortCircuit: true,
      source: `export default new WebAssembly.Module(Uint8Array.from(atob('${bytes.toString('base64')}'), (c) => c.charCodeAt(0)));`,
    };
  }
  return siguiente(url, contexto);
}
