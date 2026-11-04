import { useQueryClient } from '@tanstack/react-query'
import { CloudUpload, TriangleAlert, WifiOff } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback/notify'
import { useSession } from '@/app/session/session-context'
import { cn } from '@/lib/utils'
import { useOnline } from './network'
import { OutboxDialog } from './OutboxDialog'
import {
  connectOutboxChannel,
  flushOutbox,
  refreshOutboxSnapshot,
  subscribeToOutboxDelivery,
  useOutboxSnapshot,
} from './outbox'
import { ownerFromMe, setCurrentOwner } from './owner'
import { persistRecordTabs } from './record-tabs'
import { claimOfflineStorage, installOfflineSignOutCleanup, watchSessionExpiry } from './session'

const FLUSH_INTERVAL_MS = 30_000
const INVALIDATED_QUERY_ROOTS = [['tasks'], ['notes'], ['records'], ['telephony']] as const

installOfflineSignOutCleanup()

function useOfflineSession(): boolean {
  const { me } = useSession()
  const owner = ownerFromMe(me)
  const userId = owner?.userId ?? null

  useEffect(() => {
    if (!userId) {
      setCurrentOwner(null)
      void refreshOutboxSnapshot()
      return
    }
    let active = true
    setCurrentOwner({ userId })
    void claimOfflineStorage(userId)
      .catch(() => undefined)
      .then(async () => {
        if (!active) return
        await refreshOutboxSnapshot()
        await flushOutbox()
      })
    return () => {
      active = false
    }
  }, [userId])

  return owner !== null
}

function useOfflineQueryCache(): void {
  const queryClient = useQueryClient()
  useEffect(() => watchSessionExpiry(queryClient), [queryClient])
  useEffect(() => persistRecordTabs(queryClient), [queryClient])
}

function useOutboxTriggers(): void {
  useEffect(() => {
    const flush = () => void flushOutbox()
    const onVisibility = () => {
      if (document.visibilityState === 'visible') flush()
    }
    const disconnect = connectOutboxChannel()
    window.addEventListener('online', flush)
    window.addEventListener('focus', flush)
    document.addEventListener('visibilitychange', onVisibility)
    const interval = window.setInterval(flush, FLUSH_INTERVAL_MS)
    return () => {
      disconnect()
      window.removeEventListener('online', flush)
      window.removeEventListener('focus', flush)
      document.removeEventListener('visibilitychange', onVisibility)
      window.clearInterval(interval)
    }
  }, [])
}

function useDeliveryFeedback(): void {
  const { t } = useTranslation('offline')
  const queryClient = useQueryClient()
  useEffect(
    () =>
      subscribeToOutboxDelivery(({ item }) => {
        notifySuccess(t('toast.sent', { label: item.label }))
        for (const queryKey of INVALIDATED_QUERY_ROOTS) void queryClient.invalidateQueries({ queryKey })
      }),
    [queryClient, t],
  )
}

export function OfflineShell() {
  const { t } = useTranslation('offline')
  const ready = useOfflineSession()
  const online = useOnline()
  const outbox = useOutboxSnapshot()
  const [open, setOpen] = useState(false)
  useOfflineQueryCache()
  useOutboxTriggers()
  useDeliveryFeedback()

  const pending = outbox.pending.length
  const conflicts = outbox.conflicts.length
  const visible = ready && (!online || pending > 0 || conflicts > 0)

  return (
    <>
      {visible && (
        <button
          type="button"
          onClick={() => setOpen(true)}
          aria-label={t('indicator.open')}
          className={cn(
            'fixed bottom-20 left-1/2 z-40 flex -translate-x-1/2 items-center gap-2 rounded-full border px-3 py-1.5 text-xs font-medium shadow-md md:bottom-4',
            conflicts > 0
              ? 'border-destructive bg-destructive-solid text-destructive-foreground'
              : 'bg-background text-foreground',
          )}
        >
          {!online && (
            <span className="flex items-center gap-1">
              <WifiOff className="size-3.5" />
              {t('indicator.offline')}
            </span>
          )}
          {pending > 0 && (
            <span className="flex items-center gap-1">
              <CloudUpload className="size-3.5" />
              {t('indicator.pending', { count: pending })}
            </span>
          )}
          {conflicts > 0 && (
            <span className="flex items-center gap-1">
              <TriangleAlert className="size-3.5" />
              {t('indicator.conflicts', { count: conflicts })}
            </span>
          )}
        </button>
      )}
      <OutboxDialog open={open && ready} onOpenChange={setOpen} snapshot={outbox} online={online} />
    </>
  )
}
