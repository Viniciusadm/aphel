import { test } from 'node:test'
import assert from 'node:assert/strict'
import { PaymentLedger } from '../src/payments.js'
const charge = (id, amount, extra = {}) => ({ sale_id: 'sale', provider: 'stone', account: 'merchant', id, revision: 1, currency: 'BRL', amount, refunded: 0, status: 'captured', ...extra })

test('split payments, repeated webhooks, excess, partial refund and stale delivery', () => {
  const ledger = new PaymentLedger('sale', 'BRL', 1000)
  ledger.apply(charge('a', 400))
  assert.equal(ledger.apply(charge('a', 400)), false)
  assert.equal(ledger.summary().balance, 600)
  assert.equal(ledger.summary().can_complete, false)
  ledger.apply(charge('b', 600))
  assert.equal(ledger.summary().can_complete, true)
  ledger.apply(charge('c', 100))
  assert.equal(ledger.summary().status, 'overpaid')
  assert.equal(ledger.summary().can_complete, false)
  ledger.apply(charge('c', 100, { revision: 2, refunded: 100 }))
  assert.equal(ledger.apply(charge('c', 100)), false)
  assert.equal(ledger.summary().can_complete, true)
  ledger.apply(charge('a', 400, { revision: 2, refunded: 200 }))
  assert.equal(ledger.summary().balance, 200)
  assert.deepEqual(PaymentLedger.fromSnapshot(JSON.parse(JSON.stringify(ledger))).summary(), ledger.summary())
})

test('pending charge blocks completion and invalid input cannot alter settlement', () => {
  const ledger = new PaymentLedger('sale', 'BRL', 1000)
  ledger.apply(charge('a', 1000))
  ledger.apply(charge('b', 500, { status: 'pending' }))
  assert.equal(ledger.summary().can_complete, false)
  ledger.apply(charge('b', 500, { status: 'canceled', revision: 2 }))
  assert.equal(ledger.summary().can_complete, true)
  for (const invalid of [charge('a', 900), charge('c', 1, { currency: 'USD' }), charge('c', 1, { sale_id: 'other' }), charge('a', 1000, { status: 'canceled', revision: 3 }), charge('d', 0.01), charge('d', Number.MAX_SAFE_INTEGER + 1)]) {
    assert.throws(() => ledger.apply(invalid))
  }
  assert.equal(ledger.summary().net, 1000)
})

test('aggregated overflow rolls back the attempted update', () => {
  const ledger = new PaymentLedger('sale', 'BRL', 1)
  ledger.apply(charge('a', Number.MAX_SAFE_INTEGER))
  assert.throws(() => ledger.apply(charge('b', 1)))
  assert.equal(ledger.snapshots().length, 1)
})
