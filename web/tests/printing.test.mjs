import { test, afterEach } from 'node:test'
import assert from 'node:assert/strict'
import { sendLocalPrintJob } from '../src/sdk/printing.ts'
const originalFetch = globalThis.fetch
const originalWindow = globalThis.window
const job = { id: 'stable-job-id', target: 'cashier', payload: '<p>receipt</p>' }
const configuration = { url: 'http://127.0.0.1:9876', token: 'local-secret' }
afterEach(() => { globalThis.fetch = originalFetch; globalThis.window = originalWindow })

test('refuses server-side or non-loopback delivery before sending a token', async () => {
  delete globalThis.window
  await assert.rejects(sendLocalPrintJob(configuration, job), { outcome: 'not_sent' })
  globalThis.window = {}
  globalThis.fetch = () => { throw new Error('must not send') }
  for (const url of ['https://evil.test', 'http://localhost.evil.test', 'http://user@localhost', 'http://localhost/path', 'http://localhost?token=x']) {
    await assert.rejects(sendLocalPrintJob({ ...configuration, url }, job), { outcome: 'not_sent' })
  }
})

test('sends a stable id and treats accepted as distinct from printed', async () => {
  globalThis.window = {}
  globalThis.fetch = async (url, options) => {
    assert.equal(url, 'http://127.0.0.1:9876/print')
    assert.equal(options.headers['Idempotency-Key'], job.id)
    assert.equal(options.headers.Authorization, 'Bearer local-secret')
    assert.equal(options.redirect, 'error')
    assert.equal(options.credentials, 'omit')
    assert.deepEqual(JSON.parse(options.body), job)
    return Response.json({ job_id: job.id, status: 'accepted' })
  }
  assert.equal((await sendLocalPrintJob(configuration, job)).status, 'accepted')
})

test('lost acknowledgements are unknown and are never retried automatically', async () => {
  globalThis.window = {}
  let attempts = 0
  globalThis.fetch = async () => { attempts++; throw new Error('connection lost after send') }
  await assert.rejects(sendLocalPrintJob(configuration, job), { outcome: 'unknown' })
  assert.equal(attempts, 1)
  globalThis.fetch = async () => Response.json({ job_id: 'different-job', status: 'printed' })
  await assert.rejects(sendLocalPrintJob(configuration, job), { outcome: 'unknown' })
})
