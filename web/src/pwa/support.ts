export const SERVICE_WORKER_URL = '/sw.js'

export function isServiceWorkerEnabled(): boolean {
  return import.meta.env.PROD && typeof navigator !== 'undefined' && 'serviceWorker' in navigator
}
