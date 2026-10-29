import { ArrowLeft, FileText, Link2, X } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { emailApi, useInvalidateThreads, useThread } from '../api'
import { LinkedRecord } from '../components/LinkedRecord'
import { VisibilityBadge } from '../components/VisibilityBadge'
import { notifyEmailError } from '../lib/errors'
import { joinParticipants, threadCounterparts } from '../lib/format'
import { useRecordTypeLabel } from '../lib/record-types'
import { EMAIL_VISIBILITIES, type EmailMessage, type EmailThreadDetail, type ThreadRecordLink } from '../types'
import type { ReplyContext } from './Composer'
import { LinkRecordDialog } from './LinkRecordDialog'
import { MessageItem } from './MessageItem'

interface ThreadReaderProps {
  threadId: string
  canSend: boolean
  onBack: () => void
  onReply: (context: ReplyContext) => void
}

export function ThreadReader({ threadId, canSend, onBack, onReply }: ThreadReaderProps) {
  const { t } = useTranslation('email')
  const thread = useThread(threadId)

  return (
    <div className="grid gap-4">
      <Button variant="ghost" size="sm" className="w-fit" onClick={onBack}>
        <ArrowLeft aria-hidden="true" />
        {t('reader.back')}
      </Button>
      {thread.isPending && (
        <div className="grid gap-3" aria-busy="true">
          <Skeleton className="h-8 w-2/3" />
          <Skeleton className="h-32 w-full" />
          <Skeleton className="h-32 w-full" />
        </div>
      )}
      {thread.isError && <ErrorState onRetry={() => void thread.refetch()} />}
      {thread.isSuccess && <ThreadContent thread={thread.data} canSend={canSend} onReply={onReply} />}
    </div>
  )
}

function ThreadContent({
  thread,
  canSend,
  onReply,
}: {
  thread: EmailThreadDetail
  canSend: boolean
  onReply: (context: ReplyContext) => void
}) {
  const { t } = useTranslation('email')
  const { me } = useSession()
  const invalidate = useInvalidateThreads()
  const isOwner = thread.owner.id === me.user.id
  const fullAccess = thread.access === 'full'
  const [linking, setLinking] = useState(false)
  const [unlinking, setUnlinking] = useState<ThreadRecordLink | null>(null)
  const [cancelling, setCancelling] = useState<EmailMessage | null>(null)
  const lastId = thread.messages.at(-1)?.id
  const replyTarget = [...thread.messages].reverse().find((message) => message.status === 'received' || message.status === 'sent')

  return (
    <div className="grid gap-4">
      <header className="grid gap-2">
        <h3 className="text-lg font-semibold break-words">{thread.subject || t('thread.noSubject')}</h3>
        <p className="text-sm break-words text-muted-foreground">{joinParticipants(threadCounterparts(thread), 6)}</p>
        <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
          <span>{t('thread.ownerMailbox', { name: thread.owner.display_name, address: thread.mailbox_address })}</span>
          <VisibilityBadge visibility={thread.visibility} />
        </div>
        {isOwner && <VisibilityControl thread={thread} />}
        {!fullAccess && (
          <p className="flex items-start gap-2 rounded-md border bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
            <FileText className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
            {t('reader.metadataNotice', { name: thread.owner.display_name })}
          </p>
        )}
        <LinkedRecords
          records={thread.records}
          editable={fullAccess}
          onAdd={() => setLinking(true)}
          onRemove={(record) => setUnlinking(record)}
        />
      </header>
      <div className="grid gap-3">
        {thread.messages.map((message) => (
          <MessageItem
            key={message.id}
            message={message}
            defaultExpanded={message.id === lastId}
            canReply={isOwner && canSend && message.id === replyTarget?.id}
            canCancel={isOwner && message.status === 'scheduled'}
            onReply={() => onReply({ thread, message })}
            onCancel={() => setCancelling(message)}
          />
        ))}
      </div>
      {fullAccess && <LinkRecordDialog thread={thread} open={linking} onOpenChange={setLinking} />}
      <ConfirmDialog
        open={unlinking !== null}
        onOpenChange={(open) => !open && setUnlinking(null)}
        title={t('links.unlinkTitle')}
        description={t('links.unlinkDescription')}
        confirmLabel={t('links.unlink')}
        onConfirm={async () => {
          if (!unlinking) return
          await emailApi.unlinkThread(thread.id, unlinking)
          await invalidate()
          notifySuccess(t('links.unlinked'))
        }}
      />
      <ConfirmDialog
        open={cancelling !== null}
        onOpenChange={(open) => !open && setCancelling(null)}
        variant="destructive"
        title={t('reader.cancelTitle')}
        description={t('reader.cancelDescription')}
        confirmLabel={t('reader.cancelScheduled')}
        notifyOnError={false}
        onConfirm={async () => {
          if (!cancelling) return
          try {
            await emailApi.cancel(cancelling.id)
          } catch (error) {
            notifyEmailError(error)
            throw error
          }
          await invalidate()
          notifySuccess(t('reader.cancelled'))
        }}
      />
    </div>
  )
}

function VisibilityControl({ thread }: { thread: EmailThreadDetail }) {
  const { t } = useTranslation('email')
  const id = useId()
  const invalidate = useInvalidateThreads()
  const [saving, setSaving] = useState(false)
  const value = thread.visibility_is_default ? 'default' : thread.visibility

  const change = async (next: string) => {
    setSaving(true)
    try {
      await emailApi.setThreadVisibility(
        thread.id,
        next === 'default' ? null : EMAIL_VISIBILITIES.find((visibility) => visibility === next) ?? null,
      )
      await invalidate()
      notifySuccess(t('reader.visibilityChanged'))
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="grid gap-1 sm:max-w-sm">
      <Label htmlFor={id} className="text-xs text-muted-foreground">
        {t('reader.visibilityLabel')}
      </Label>
      <NativeSelect id={id} value={value} disabled={saving} onChange={(event) => void change(event.target.value)}>
        <option value="default">{t('reader.visibilityDefault')}</option>
        {EMAIL_VISIBILITIES.map((visibility) => (
          <option key={visibility} value={visibility}>
            {t(`visibility.${visibility}.label`)}
          </option>
        ))}
      </NativeSelect>
      <p className="text-xs text-muted-foreground">{t(`visibility.${thread.visibility}.description`)}</p>
    </div>
  )
}

function LinkedRecords({
  records,
  editable,
  onAdd,
  onRemove,
}: {
  records: ThreadRecordLink[]
  editable: boolean
  onAdd: () => void
  onRemove: (record: ThreadRecordLink) => void
}) {
  const { t } = useTranslation('email')
  const recordTypeLabel = useRecordTypeLabel()
  return (
    <div className="flex flex-wrap items-center gap-1.5 text-sm">
      <span className="text-xs text-muted-foreground">{t('links.linkedTo')}</span>
      {records.length === 0 && <span className="text-xs text-muted-foreground">{t('links.none')}</span>}
      {records.map((record) => (
        <Badge key={`${record.record_type}:${record.record_id}`} variant="outline" className="max-w-full gap-1.5 py-1">
          <span className="text-muted-foreground">{recordTypeLabel(record.record_type)}:</span>
          <LinkedRecord record_type={record.record_type} record_id={record.record_id} />
          {record.origin === 'automatic' && (
            <span className="text-muted-foreground" title={t('links.automaticHint')}>
              {t('links.automatic')}
            </span>
          )}
          {editable && (
            <button
              type="button"
              className="rounded-sm opacity-70 hover:opacity-100 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              aria-label={t('links.unlink')}
              onClick={() => onRemove(record)}
            >
              <X className="size-3" />
            </button>
          )}
        </Badge>
      ))}
      {editable && (
        <Button variant="ghost" size="sm" className="h-7" onClick={onAdd}>
          <Link2 aria-hidden="true" />
          {t('links.add')}
        </Button>
      )}
    </div>
  )
}
