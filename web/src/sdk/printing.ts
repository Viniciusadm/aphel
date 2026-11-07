/** Browser-to-loopback transport. Backend jobs must never call the cashier's localhost. */
export interface PrintBridgeConfiguration {
  url: string
  token: string
  timeoutMs?: number
}

export interface LocalPrintJob {
  /** Persisted, stable id. Retry the same job with the same id. */
  id: string
  target: string
  payload: string
}

export interface PrintAcknowledgement {
  job_id: string
  /** Accepted/duplicate is not proof that paper was physically printed. */
  status: 'accepted' | 'printed' | 'duplicate'
}

export class PrintDeliveryError extends Error {
  readonly outcome: 'not_sent' | 'rejected' | 'unknown'
  constructor(outcome: PrintDeliveryError['outcome'], message: string) {
    super(message)
    this.name = 'PrintDeliveryError'
    this.outcome = outcome
  }
}

function endpoint(configuration: PrintBridgeConfiguration): string {
  let url: URL
  try { url = new URL(configuration.url) } catch {
    throw new PrintDeliveryError('not_sent', 'Invalid print bridge URL')
  }
  if (!['http:', 'https:'].includes(url.protocol)
    || !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
    || url.username || url.password || url.search || url.hash || url.pathname !== '/') {
    throw new PrintDeliveryError('not_sent', 'Print bridge must use an exact loopback origin')
  }
  if (!configuration.token || /[\r\n]/.test(configuration.token)) {
    throw new PrintDeliveryError('not_sent', 'A local print bridge token is required')
  }
  return new URL('/print', url).href
}

/** Does not retry or open the browser print dialog after an ambiguous delivery. */
export async function sendLocalPrintJob(
  configuration: PrintBridgeConfiguration,
  job: LocalPrintJob,
): Promise<PrintAcknowledgement> {
  if (typeof window === 'undefined') {
    throw new PrintDeliveryError('not_sent', 'Local printing must run in the cashier browser')
  }
  const url = endpoint(configuration)
  if (!job.id || job.id.length > 128 || !job.target || !job.payload) {
    throw new PrintDeliveryError('not_sent', 'Print job id, target and payload are required')
  }
  const timeout = configuration.timeoutMs ?? 5000
  if (!Number.isFinite(timeout) || timeout < 1 || timeout > 60000) {
    throw new PrintDeliveryError('not_sent', 'Print bridge timeout must be between 1 and 60000 ms')
  }
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeout)
  try {
    const response = await fetch(url, {
      method: 'POST',
      credentials: 'omit',
      cache: 'no-store',
      redirect: 'error',
      referrerPolicy: 'no-referrer',
      signal: controller.signal,
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${configuration.token}`,
        'Idempotency-Key': job.id,
      },
      body: JSON.stringify(job),
    })
    if (response.status === 401 || response.status === 403) {
      throw new PrintDeliveryError('rejected', 'Print bridge refused authentication')
    }
    if (!response.ok) throw new PrintDeliveryError('unknown', 'Print bridge did not confirm delivery')
    const acknowledgement: unknown = await response.json()
    if (!acknowledgement || typeof acknowledgement !== 'object'
      || !('job_id' in acknowledgement) || acknowledgement.job_id !== job.id
      || !('status' in acknowledgement)
      || !['accepted', 'printed', 'duplicate'].includes(String(acknowledgement.status))) {
      throw new PrintDeliveryError('unknown', 'Invalid print acknowledgement')
    }
    return acknowledgement as PrintAcknowledgement
  } catch (error) {
    if (error instanceof PrintDeliveryError) throw error
    // The printer may have received the job even when its acknowledgement was lost.
    throw new PrintDeliveryError('unknown', 'Print delivery outcome is unknown; check the job before retrying')
  } finally {
    clearTimeout(timer)
  }
}
