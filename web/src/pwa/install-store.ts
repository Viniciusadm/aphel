interface InstallPromptEvent extends Event {
  prompt: () => Promise<void>
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>
}

type Listener = () => void

let deferredPrompt: InstallPromptEvent | null = null
const listeners = new Set<Listener>()

function setPrompt(event: InstallPromptEvent | null) {
  deferredPrompt = event
  for (const listener of listeners) listener()
}

export function captureInstallPrompt(): void {
  window.addEventListener('beforeinstallprompt', (event) => {
    event.preventDefault()
    setPrompt(event as InstallPromptEvent)
  })
  window.addEventListener('appinstalled', () => setPrompt(null))
}

export function subscribeToInstallPrompt(listener: Listener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function canPromptInstall(): boolean {
  return deferredPrompt !== null
}

export async function promptInstall(): Promise<boolean> {
  const event = deferredPrompt
  if (!event) return false
  setPrompt(null)
  await event.prompt()
  const choice = await event.userChoice
  return choice.outcome === 'accepted'
}
