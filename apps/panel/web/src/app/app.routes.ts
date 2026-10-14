import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'equipos' },
  {
    path: 'equipos',
    title: 'Equipos · Limen',
    loadComponent: () => import('./equipos/equipos-pagina').then((m) => m.EquiposPagina),
  },
  // Una dirección que no existe vuelve a la primera sección.
  { path: '**', redirectTo: 'equipos' },
];
