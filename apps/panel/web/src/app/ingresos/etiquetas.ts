import type { Ingreso, ViaDeIngreso } from '../panel';

/** Las vías, en el orden en que se ofrecen como filtro. */
export const VIAS: readonly { codigo: ViaDeIngreso | ''; etiqueta: string }[] = [
  { codigo: '', etiqueta: 'Todas' },
  { codigo: 'CONTRATISTA', etiqueta: 'Contratistas' },
  { codigo: 'PROVEEDOR', etiqueta: 'Proveedores' },
  { codigo: 'CORREO', etiqueta: 'Correo' },
  { codigo: 'KOF', etiqueta: 'KOF' },
];

const UNA_VIA: Record<ViaDeIngreso, string> = {
  CONTRATISTA: 'Contratista',
  PROVEEDOR: 'Proveedor',
  CORREO: 'Correo',
  KOF: 'KOF',
};

export const etiquetaDeVia = (via: ViaDeIngreso): string => UNA_VIA[via];

/** «A pie», «Vehículo ABC-123» o vacío si no se registra (el personal KOF). */
export function textoMedio(ingreso: Pick<Ingreso, 'via' | 'placa'>): string {
  if (ingreso.via === 'KOF') {
    return '';
  }
  return ingreso.placa ? `Vehículo ${ingreso.placa}` : 'A pie';
}

/** El número de gafete, «S/G» si entró sin gafete, o «—» si no le aplica. */
export function textoGafete(ingreso: Pick<Ingreso, 'gafete' | 'sin_gafete'>): string {
  if (ingreso.gafete !== null) {
    return String(ingreso.gafete);
  }
  return ingreso.sin_gafete ? 'S/G' : '—';
}

/** Quién lo registró: su nombre, o «—» si la nube no conoce a ese usuario. */
export const textoOperador = (nombre: string | null): string => nombre ?? '—';
