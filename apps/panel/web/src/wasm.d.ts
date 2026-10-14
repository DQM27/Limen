// Con `"loader": { ".wasm": "file" }` (angular.json), importar un `.wasm` da la ruta con que se sirve.
declare module '*.wasm' {
  const ruta: string;
  export default ruta;
}
