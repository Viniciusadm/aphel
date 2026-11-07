export interface ChargeSnapshot {
  sale_id: string
  provider: string
  account: string
  id: string
  revision: number
  currency: string
  /** Integer minor units. Never pass decimal currency amounts. */
  amount: number
  refunded: number
  status: 'pending' | 'captured' | 'failed' | 'canceled'
}
export interface Settlement {
  status: 'unpaid' | 'partial' | 'pending' | 'paid' | 'overpaid' | 'refunded'
  captured: number
  refunded: number
  net: number
  balance: number
  excess: number
  pending: number
  can_complete: boolean
}
export interface StoredPaymentLedger { sale_id: string; currency: string; total: number; charges: ChargeSnapshot[] }
export class PaymentLedger {
  static fromSnapshot(value: StoredPaymentLedger): PaymentLedger
  toJSON(): StoredPaymentLedger
  constructor(sale_id: string, currency: string, total: number)
  snapshots(): ChargeSnapshot[]
  apply(snapshot: ChargeSnapshot): boolean
  summary(): Settlement
}
