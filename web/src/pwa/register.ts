import { captureInstallPrompt } from './install-store'
import { SERVICE_WORKER_URL, isServiceWorkerEnabled, serviceWorkerScope } from './support'
import { trackRegistration } from './update-store'

const LEGACY_SERVICE_WORKER_PATH = '/sw.js'

function isAphelWorker(registration: ServiceWorkerRegistration): boolean {
  const script = registration.active ?? registration.waiting ?? registration.installing
  if (!script) return false
  const path = new URL(script.scriptURL).pathname
  return path === SERVICE_WORKER_URL || path === LEGACY_SERVICE_WORKER_PATH
}

async function removeWorkersOfOtherMounts(scope: string): Promise<void> {
  const registrations = await navigator.serviceWorker.getRegistrations()
  await Promise.all(
    registrations
      .filter((registration) => registration.scope !== scope && isAphelWorker(registration))
      .map((registration) => registration.unregister()),
  )
}

export function registerPwa(): void {
  if (!isServiceWorkerEnabled()) return
  captureInstallPrompt()
  const register = () => {
    const scope = new URL(serviceWorkerScope(), window.location.origin).href
    navigator.serviceWorker
      .register(SERVICE_WORKER_URL, { scope: serviceWorkerScope() })
      .then(trackRegistration)
      .then(() => removeWorkersOfOtherMounts(scope))
      .catch(() => undefined)
  }
  if (document.readyState === 'complete') register()
  else window.addEventListener('load', register, { once: true })
}
