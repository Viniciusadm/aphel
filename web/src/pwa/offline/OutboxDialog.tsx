import { Copy, RotateCcw, Send, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback/ConfirmDialog'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import {
  REPLAY_WINDOW_EXPIRED,
  discardOutboxItem,
  flushOutbox,
  outboxItemText,
  retryOutboxItem,
  type OutboxItem,
  type OutboxSnapshot,
} from './outbox'

export interface OutboxDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  snapshot: OutboxSnapshot
  online: boolean
}

type ConflictReason = 'expired' | 'validation' | 'forbidden' | 'notFound' | 'conflict' | 'generic'

function conflictReason(item: OutboxItem): ConflictReason {
  if (item.error?.code === REPLAY_WINDOW_EXPIRED) return 'expired'
  switch (item.error?.status) {
    case 400:
    case 422:
      return 'validation'
    case 403:
      return 'forbidden'
    case 404:
      return 'notFound'
    case 409:
      return 'conflict'
    default:
      return 'generic'
  }
}

function useDateTimeFormat() {
  const { i18n } = useTranslation()
  const formatter = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' })
  return (timestamp: number) => formatter.format(new Date(timestamp))
}

function PendingItem({ item }: { item: OutboxItem }) {
  const { t } = useTranslation('offline')
  const format = useDateTimeFormat()
  return (
    <li className="flex flex-col gap-0.5 rounded-md border p-3">
      <span className="text-sm font-medium">{item.label}</span>
      <span className="text-xs text-muted-foreground">
        {t(`kinds.${item.kind}` as "kinds.note", { defaultValue: item.kind })} · {t('pending.createdAt', { date: format(item.createdAt) })}
      </span>
      {item.attempts > 0 && (
        <span className="text-xs text-muted-foreground">
          {t('pending.nextAttempt', { date: format(item.nextAttemptAt) })}
        </span>
      )}
    </li>
  )
}

function ConflictItem({ item }: { item: OutboxItem }) {
  const { t } = useTranslation('offline')
  const format = useDateTimeFormat()
  const [confirming, setConfirming] = useState(false)
  const [busy, setBusy] = useState(false)
  const reason = conflictReason(item)
  const text = outboxItemText(item)

  const retry = async () => {
    setBusy(true)
    try {
      await retryOutboxItem(item.id)
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      notifySuccess(t('conflicts.copied'))
    } catch (error) {
      notifyError(error)
    }
  }

  return (
    <li className="flex flex-col gap-2 rounded-md border border-destructive/40 p-3">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm font-medium">{item.label}</span>
        <span className="text-xs text-muted-foreground">
          {t(`kinds.${item.kind}` as "kinds.note", { defaultValue: item.kind })} · {t('pending.createdAt', { date: format(item.createdAt) })}
        </span>
        <span className="text-sm text-destructive">{t(`conflicts.reasons.${reason}`)}</span>
        {reason === 'validation' && item.error?.message && (
          <span className="text-xs text-muted-foreground">{item.error.message}</span>
        )}
      </div>
      {text && <p className="line-clamp-3 text-xs whitespace-pre-wrap text-muted-foreground">{text}</p>}
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="outline" disabled={busy} onClick={() => void retry()}>
          <RotateCcw />
          {t('conflicts.retry')}
        </Button>
        {text && (
          <Button size="sm" variant="ghost" onClick={() => void copy()}>
            <Copy />
            {t('conflicts.copy')}
          </Button>
        )}
        <Button size="sm" variant="ghost" className="text-destructive" onClick={() => setConfirming(true)}>
          <Trash2 />
          {t('conflicts.discard')}
        </Button>
      </div>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('conflicts.discardTitle')}
        description={t('conflicts.discardDescription', { label: item.label })}
        confirmLabel={t('conflicts.discard')}
        variant="destructive"
        onConfirm={() => discardOutboxItem(item.id)}
      />
    </li>
  )
}

export function OutboxDialog({ open, onOpenChange, snapshot, online }: OutboxDialogProps) {
  const { t } = useTranslation('offline')
  const [sending, setSending] = useState(false)
  const empty = snapshot.pending.length === 0 && snapshot.conflicts.length === 0

  const sendNow = async () => {
    setSending(true)
    try {
      await flushOutbox({ force: true })
    } finally {
      setSending(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('dialog.title')}</DialogTitle>
          <DialogDescription>{online ? t('dialog.onlineDescription') : t('dialog.offlineDescription')}</DialogDescription>
        </DialogHeader>
        {empty && <p className="text-sm text-muted-foreground">{t('dialog.empty')}</p>}
        {snapshot.conflicts.length > 0 && (
          <section className="flex flex-col gap-2">
            <h3 className="text-sm font-semibold">{t('conflicts.title')}</h3>
            <p className="text-xs text-muted-foreground">{t('conflicts.description')}</p>
            <ul className="flex flex-col gap-2">
              {snapshot.conflicts.map((item) => (
                <ConflictItem key={item.id} item={item} />
              ))}
            </ul>
          </section>
        )}
        {snapshot.pending.length > 0 && (
          <section className="flex flex-col gap-2">
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-sm font-semibold">{t('pending.title')}</h3>
              <Button size="sm" variant="outline" disabled={!online || sending} onClick={() => void sendNow()}>
                <Send />
                {t('pending.sendNow')}
              </Button>
            </div>
            <ul className="flex flex-col gap-2">
              {snapshot.pending.map((item) => (
                <PendingItem key={item.id} item={item} />
              ))}
            </ul>
          </section>
        )}
      </DialogContent>
    </Dialog>
  )
}
