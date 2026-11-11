import { appBase, onSiteSurface } from '@/sdk/app-base'

export const SERVICE_WORKER_URL = '/_aphel/sw.js'

export function serviceWorkerScope(): string {
  return `${appBase()}/`
}

export function isServiceWorkerEnabled(): boolean {
  return import.meta.env.PROD && !onSiteSurface() && typeof navigator !== 'undefined' && 'serviceWorker' in navigator
}
