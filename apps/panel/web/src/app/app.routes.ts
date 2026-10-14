import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'equipos' },
  {
    path: 'equipos',
    title: 'Equipos · Limen',
    loadComponent: () => import('./equipos/equipos-pagina').then((m) => m.EquiposPagina),
  },
  {
    path: 'usuarios',
    title: 'Usuarios · Limen',
    loadComponent: () => import('./usuarios/usuarios-pagina').then((m) => m.UsuariosPagina),
  },
  {
    path: 'ingresos',
    title: 'Ingresos · Limen',
    loadComponent: () => import('./ingresos/ingresos-pagina').then((m) => m.IngresosPagina),
  },
  {
    path: 'contratistas',
    title: 'Contratistas · Limen',
    loadComponent: () =>
      import('./contratistas/contratistas-pagina').then((m) => m.ContratistasPagina),
  },
  // Una dirección que no existe vuelve a la primera sección.
  { path: '**', redirectTo: 'equipos' },
];
