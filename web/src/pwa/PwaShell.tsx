import { RefreshCw, X } from 'lucide-react'
import { useEffect, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { syncPushSubscription } from './push/push'
import { isServiceWorkerEnabled } from './support'
import { applyUpdate, getWaitingWorker, subscribeToUpdate } from './update-store'

interface OpenUrlMessage {
  type: 'OPEN_URL'
  url: string
}

function isOpenUrlMessage(data: unknown): data is OpenUrlMessage {
  if (typeof data !== 'object' || data === null) return false
  const message = data as Partial<OpenUrlMessage>
  return message.type === 'OPEN_URL' && typeof message.url === 'string' && message.url.startsWith('/')
}

export function PwaShell() {
  const { t } = useTranslation('pwa')
  const navigate = useNavigate()
  const waiting = useSyncExternalStore(subscribeToUpdate, getWaitingWorker, () => null)
  const [dismissed, setDismissed] = useState<ServiceWorker | null>(null)
  const [updating, setUpdating] = useState(false)

  useEffect(() => {
    void syncPushSubscription().catch(() => undefined)
  }, [])

  useEffect(() => {
    if (!isServiceWorkerEnabled()) return
    const onMessage = (event: MessageEvent) => {
      if (isOpenUrlMessage(event.data)) void navigate(event.data.url)
    }
    navigator.serviceWorker.addEventListener('message', onMessage)
    return () => navigator.serviceWorker.removeEventListener('message', onMessage)
  }, [navigate])

  if (!waiting || dismissed === waiting) return null

  const update = () => {
    setUpdating(true)
    applyUpdate()
  }

  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed inset-x-4 top-4 z-50 flex items-start gap-3 rounded-lg border bg-card p-4 text-card-foreground shadow-lg md:inset-x-auto md:top-auto md:right-6 md:bottom-6 md:w-96"
    >
      <RefreshCw className="mt-0.5 size-5 shrink-0 text-primary" aria-hidden="true" />
      <div className="grid min-w-0 flex-1 gap-1">
        <p className="text-sm font-medium">{t('update.title')}</p>
        <p className="text-xs text-muted-foreground">{t('update.description')}</p>
        <div className="mt-2 flex gap-2">
          <Button size="sm" onClick={update} disabled={updating}>
            {updating ? t('update.updating') : t('update.action')}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setDismissed(waiting)} disabled={updating}>
            {t('update.later')}
          </Button>
        </div>
      </div>
      <Button
        size="icon"
        variant="ghost"
        className="-mt-1 -mr-1 size-7"
        aria-label={t('update.dismiss')}
        onClick={() => setDismissed(waiting)}
      >
        <X />
      </Button>
    </div>
  )
}
