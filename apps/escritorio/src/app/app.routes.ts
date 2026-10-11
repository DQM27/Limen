import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'contratistas' },
  {
    path: 'contratistas',
    title: 'Contratistas · Limen',
    loadComponent: () =>
      import('./contratistas/contratistas-pagina').then((m) => m.ContratistasPagina),
  },
  {
    path: 'gafetes',
    title: 'Gafetes · Limen',
    loadComponent: () => import('./gafetes/gafetes-pagina').then((m) => m.GafetesPagina),
  },
  { path: '**', redirectTo: 'contratistas' },
];
