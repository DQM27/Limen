import {
  ApplicationConfig,
  provideAppInitializer,
  provideBrowserGlobalErrorListeners,
} from '@angular/core';
import { provideRouter } from '@angular/router';
import { routes } from './app.routes';
import { Nucleo } from './nucleo/nucleo';
import { NucleoWasm, iniciarNucleo } from './nucleo/nucleo-wasm';

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    provideRouter(routes),
    // El núcleo se carga antes de mostrar nada: ninguna pantalla decide por su cuenta.
    provideAppInitializer(iniciarNucleo),
    { provide: Nucleo, useClass: NucleoWasm },
  ],
};
