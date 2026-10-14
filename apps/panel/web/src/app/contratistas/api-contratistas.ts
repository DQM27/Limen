import { Injectable } from '@angular/core';
import {
  crearEmpresa,
  editarContratista,
  listarContratistas,
  registrarContratista,
  type Contratista,
  type EmpresaCatalogo,
  type FormularioContratista,
} from '../panel';

/**
 * Lo que la pantalla de contratistas le pide al Worker. Es un servicio para que las pruebas lo
 * reemplacen por uno falso; no decide nada.
 */
@Injectable({ providedIn: 'root' })
export class ApiContratistas {
  listar(): Promise<{ contratistas: Contratista[]; empresas: EmpresaCatalogo[] }> {
    return listarContratistas();
  }

  registrar(datos: FormularioContratista): Promise<Contratista> {
    return registrarContratista(datos);
  }

  editar(id: string, datos: FormularioContratista): Promise<Contratista> {
    return editarContratista(id, datos);
  }

  crearEmpresa(nombre: string): Promise<EmpresaCatalogo> {
    return crearEmpresa(nombre);
  }
}
