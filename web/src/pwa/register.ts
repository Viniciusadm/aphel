import { captureInstallPrompt } from './install-store'
import { forgetPushOnThisDevice } from './push/push'
import { registerSignOutCleanup } from './sign-out'
import { SERVICE_WORKER_URL, isServiceWorkerEnabled } from './support'
import { trackRegistration } from './update-store'

export function registerPwa(): void {
  if (!isServiceWorkerEnabled()) return
  captureInstallPrompt()
  registerSignOutCleanup(forgetPushOnThisDevice)
  const register = () => {
    navigator.serviceWorker
      .register(SERVICE_WORKER_URL, { scope: '/' })
      .then(trackRegistration)
      .catch(() => undefined)
  }
  if (document.readyState === 'complete') register()
  else window.addEventListener('load', register, { once: true })
}
