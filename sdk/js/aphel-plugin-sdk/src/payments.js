/** Pure rules; callers authenticate provider state and persist with optimistic concurrency. */
function invalid(message) { return Object.assign(new Error(message), { code: 'payment_snapshot_invalid', status: 422 }) }
function conflict(message) { return Object.assign(new Error(message), { code: 'payment_snapshot_conflict', status: 409 }) }
const key = (value) => typeof value === 'string' && value.length > 0 && new TextEncoder().encode(value).length <= 191
const money = (value) => Number.isSafeInteger(value) && value >= 0

export class PaymentLedger {
  #sale
  #currency
  #total
  #charges = new Map()
  constructor(sale_id, currency, total) {
    if (!key(sale_id) || typeof currency !== 'string' || !/^[A-Z]{3}$/.test(currency) || !money(total)) throw invalid('Invalid sale, currency or total')
    this.#sale = sale_id; this.#currency = currency; this.#total = total
  }
  static fromSnapshot(value) {
    if (!value || !Array.isArray(value.charges) || value.charges.length > 1000) throw invalid('Invalid stored payment ledger')
    const ledger = new PaymentLedger(value.sale_id, value.currency, value.total)
    for (const charge of value.charges) ledger.apply(charge)
    return ledger
  }
  toJSON() { return { sale_id: this.#sale, currency: this.#currency, total: this.#total, charges: this.snapshots() } }
  snapshots() { return [...this.#charges.values()].map((charge) => ({ ...charge })) }
  apply(input) {
    if (!input || input.sale_id !== this.#sale || input.currency !== this.#currency
      || !key(input.provider) || !key(input.account) || !key(input.id)
      || !Number.isSafeInteger(input.revision) || input.revision < 1
      || !money(input.amount) || input.amount === 0 || !money(input.refunded) || input.refunded > input.amount
      || !['pending', 'captured', 'failed', 'canceled'].includes(input.status)
      || (input.status !== 'captured' && input.refunded !== 0)) throw invalid('Invalid charge snapshot or sale/currency mismatch')
    const { sale_id, provider, account, id, revision, currency, amount, refunded, status } = input
    const snapshot = { sale_id, provider, account, id, revision, currency, amount, refunded, status }
    const identity = JSON.stringify([provider, account, id])
    const previous = this.#charges.get(identity)
    if (previous) {
      if (revision < previous.revision) return false
      if (revision === previous.revision) {
        if (JSON.stringify(previous) === JSON.stringify(snapshot)) return false
        throw conflict('Same revision has different charge data')
      }
      if (previous.status === 'captured' && (status !== 'captured' || amount !== previous.amount || refunded < previous.refunded)) throw conflict('Captured funds or refunds cannot be erased')
    } else if (this.#charges.size >= 1000) throw invalid('Too many charges in one sale')
    this.#charges.set(identity, snapshot)
    try { this.summary() } catch (error) {
      if (previous) this.#charges.set(identity, previous)
      else this.#charges.delete(identity)
      throw error
    }
    return true
  }
  summary() {
    let captured = 0; let refunded = 0; let pending = 0
    for (const charge of this.#charges.values()) {
      if (charge.status === 'pending') pending++
      if (charge.status === 'captured') { captured += charge.amount; refunded += charge.refunded }
      if (!money(captured) || !money(refunded)) throw invalid('Payment total overflow')
    }
    const net = captured - refunded
    const balance = Math.max(this.#total - net, 0)
    const excess = Math.max(net - this.#total, 0)
    const can_complete = net === this.#total && pending === 0
    const status = excess > 0 ? 'overpaid' : pending > 0 ? 'pending' : can_complete ? 'paid' : captured > 0 && net === 0 ? 'refunded' : net > 0 ? 'partial' : 'unpaid'
    return { status, captured, refunded, net, balance, excess, pending, can_complete }
  }
}
