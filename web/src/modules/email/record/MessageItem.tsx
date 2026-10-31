import { ChevronDown, ChevronUp, Download, Eye, MousePointerClick, Paperclip, Reply, Undo2, XCircle } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { attachmentDownloadUrl } from '../api'
import { HtmlFrame, TextBody } from '../components/HtmlFrame'
import { formatFileSize, joinParticipants, participantName, participantsByRole } from '../lib/format'
import type { EmailMessage, EmailStatus } from '../types'
import { Alert } from '@/components/ui/alert'

const STATUS_VARIANTS: Partial<Record<EmailStatus, BadgeProps['variant']>> = {
  draft: 'outline',
  scheduled: 'secondary',
  sending: 'secondary',
  failed: 'destructive',
  cancelled: 'warning',
}

interface MessageItemProps {
  message: EmailMessage
  defaultExpanded: boolean
  canReply: boolean
  canCancel: boolean
  onReply: () => void
  onCancel: () => void
}

export function MessageItem({ message, defaultExpanded, canReply, canCancel, onReply, onCancel }: MessageItemProps) {
  const { t } = useTranslation('email')
  const formatting = useRecordFormatting()
  const [expanded, setExpanded] = useState(defaultExpanded)
  const from = participantsByRole(message.participants, 'from')[0]
  const to = participantsByRole(message.participants, 'to')
  const cc = participantsByRole(message.participants, 'cc')
  const bcc = participantsByRole(message.participants, 'bcc')
  const date = message.sent_at ?? message.scheduled_at
  const statusVariant = STATUS_VARIANTS[message.status]
  const attachments = message.attachments.filter((attachment) => !attachment.inline)

  return (
    <article className="grid gap-3 rounded-lg border p-3">
      <button
        type="button"
        className="grid gap-0.5 text-left focus-visible:outline-none"
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
      >
        <div className="flex items-start gap-2">
          <p className="min-w-0 flex-1 truncate text-sm font-medium" title={from?.address}>
            {from ? participantName(from) : t('reader.unknownSender')}
          </p>
          {date && (
            <time dateTime={date} className="shrink-0 text-xs text-muted-foreground">
              {formatDateTime(date, formatting)}
            </time>
          )}
          {expanded ? (
            <ChevronUp className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          ) : (
            <ChevronDown className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          )}
        </div>
        {to.length > 0 && (
          <p className="truncate text-xs text-muted-foreground">{t('reader.toLine', { names: joinParticipants(to) })}</p>
        )}
        {!expanded && message.preview && <p className="line-clamp-1 text-sm text-muted-foreground">{message.preview}</p>}
      </button>
      <div className="flex flex-wrap gap-1.5">
        {statusVariant && (
          <Badge variant={statusVariant}>{t(`messageStatus.${message.status}`)}</Badge>
        )}
        {message.kind !== 'message' && <Badge variant="outline">{t(`messageKind.${message.kind}`)}</Badge>}
      </div>
      {expanded && (
        <>
          {(cc.length > 0 || bcc.length > 0) && (
            <div className="grid gap-0.5 text-xs text-muted-foreground">
              {cc.length > 0 && <p className="break-words">{t('reader.ccLine', { names: joinParticipants(cc, 10) })}</p>}
              {bcc.length > 0 && <p className="break-words">{t('reader.bccLine', { names: joinParticipants(bcc, 10) })}</p>}
            </div>
          )}
          {message.status === 'failed' && message.failure_reason && (
            <Alert className="flex items-start gap-2 text-destructive">
              <XCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              <span className="break-words">{message.failure_reason}</span>
            </Alert>
          )}
          {message.body?.html ? (
            <HtmlFrame html={message.body.html} title={message.subject ?? t('reader.bodyTitle')} />
          ) : message.body?.text ? (
            <TextBody text={message.body.text} />
          ) : message.body ? (
            <p className="text-sm text-muted-foreground italic">{t('reader.emptyBody')}</p>
          ) : null}
          {attachments.length > 0 && (
            <ul className="flex flex-wrap gap-2">
              {attachments.map((attachment) => (
                <li key={attachment.id}>
                  <a
                    href={attachmentDownloadUrl(message.id, attachment.id)}
                    download={attachment.name}
                    className="inline-flex max-w-full items-center gap-1.5 rounded-md border px-2 py-1 text-xs hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
                  >
                    <Paperclip className="size-3.5 shrink-0" aria-hidden="true" />
                    <span className="truncate">{attachment.name}</span>
                    <span className="shrink-0 text-muted-foreground">{formatFileSize(attachment.size, formatting.locale)}</span>
                    <Download className="size-3.5 shrink-0" aria-hidden="true" />
                  </a>
                </li>
              ))}
            </ul>
          )}
          {message.tracking && message.direction === 'outbound' && <TrackingInfo message={message} />}
          {(canReply || canCancel) && (
            <div className="flex flex-wrap gap-2">
              {canReply && (
                <Button variant="outline" size="sm" onClick={onReply}>
                  <Reply aria-hidden="true" />
                  {t('reader.reply')}
                </Button>
              )}
              {canCancel && (
                <Button variant="outline" size="sm" onClick={onCancel}>
                  <Undo2 aria-hidden="true" />
                  {t('reader.cancelScheduled')}
                </Button>
              )}
            </div>
          )}
        </>
      )}
    </article>
  )
}

function TrackingInfo({ message }: { message: EmailMessage }) {
  const { t } = useTranslation('email')
  const formatting = useRecordFormatting()
  const tracking = message.tracking
  if (!tracking) return null
  if (!tracking.enabled) return <p className="text-xs text-muted-foreground">{t('tracking.disabledForMessage')}</p>
  const items = [
    {
      key: 'opened',
      icon: Eye,
      text: tracking.opened_at
        ? t('tracking.opened', { count: tracking.open_count, date: formatDateTime(tracking.opened_at, formatting) })
        : t('tracking.notOpened'),
    },
    {
      key: 'clicked',
      icon: MousePointerClick,
      text: tracking.clicked_at
        ? t('tracking.clicked', { count: tracking.click_count, date: formatDateTime(tracking.clicked_at, formatting) })
        : t('tracking.notClicked'),
    },
  ]
  return (
    <div className="grid gap-1 rounded-md bg-muted/50 px-3 py-2 text-xs text-muted-foreground">
      <p className="font-medium text-foreground">{t('tracking.ownerOnly')}</p>
      {items.map(({ key, icon: Icon, text }) => (
        <p key={key} className="flex items-center gap-1.5">
          <Icon className="size-3.5" aria-hidden="true" />
          {text}
        </p>
      ))}
      {tracking.replied_at && <p>{t('tracking.replied', { date: formatDateTime(tracking.replied_at, formatting) })}</p>}
      {tracking.bounced_at && (
        <p className="text-destructive">{t('tracking.bounced', { date: formatDateTime(tracking.bounced_at, formatting) })}</p>
      )}
    </div>
  )
}
