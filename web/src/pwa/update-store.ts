type Listener = () => void

const UPDATE_CHECK_INTERVAL_MS = 60 * 60 * 1000

let waitingWorker: ServiceWorker | null = null
const listeners = new Set<Listener>()

function setWaiting(worker: ServiceWorker | null) {
  waitingWorker = worker
  for (const listener of listeners) listener()
}

export function subscribeToUpdate(listener: Listener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function getWaitingWorker(): ServiceWorker | null {
  return waitingWorker
}

function watchInstalling(worker: ServiceWorker) {
  worker.addEventListener('statechange', () => {
    if (worker.state === 'installed' && navigator.serviceWorker.controller) setWaiting(worker)
  })
}

export function trackRegistration(registration: ServiceWorkerRegistration): void {
  if (registration.waiting && navigator.serviceWorker.controller) setWaiting(registration.waiting)
  if (registration.installing) watchInstalling(registration.installing)
  registration.addEventListener('updatefound', () => {
    if (registration.installing) watchInstalling(registration.installing)
  })
  const checkForUpdate = () => {
    void registration.update().catch(() => undefined)
  }
  window.setInterval(checkForUpdate, UPDATE_CHECK_INTERVAL_MS)
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') checkForUpdate()
  })
}

export function applyUpdate(): void {
  const worker = waitingWorker
  if (!worker) {
    window.location.reload()
    return
  }
  let reloaded = false
  navigator.serviceWorker.addEventListener('controllerchange', () => {
    if (reloaded) return
    reloaded = true
    window.location.reload()
  })
  worker.postMessage({ type: 'SKIP_WAITING' })
}
