import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'contratistas' },
  {
    path: 'dentro',
    title: 'Dentro · Limen',
    loadComponent: () => import('./dentro/dentro-pagina').then((m) => m.DentroPagina),
  },
  {
    path: 'contratistas',
    title: 'Contratistas · Limen',
    loadComponent: () =>
      import('./contratistas/contratistas-pagina').then((m) => m.ContratistasPagina),
  },
  {
    path: 'proveedores',
    title: 'Proveedores · Limen',
    loadComponent: () =>
      import('./proveedores/proveedores-pagina').then((m) => m.ProveedoresPagina),
  },
  {
    path: 'correo',
    title: 'Correo · Limen',
    loadComponent: () => import('./correo/correo-pagina').then((m) => m.CorreoPagina),
  },
  {
    path: 'kof',
    title: 'KOF · Limen',
    loadComponent: () => import('./kof/kof-pagina').then((m) => m.KofPagina),
  },
  {
    path: 'gafetes',
    title: 'Gafetes · Limen',
    loadComponent: () => import('./gafetes/gafetes-pagina').then((m) => m.GafetesPagina),
  },
  {
    path: 'historial',
    title: 'Historial · Limen',
    loadComponent: () => import('./historial/historial-pagina').then((m) => m.HistorialPagina),
  },
  { path: '**', redirectTo: 'contratistas' },
];
