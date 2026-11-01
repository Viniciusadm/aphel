import { PaginationSummary } from '@/components/ui/pagination-summary'
import { Inbox, MailPlus, Plug } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Pagination } from '@/components/ui/pagination'
import { THREADS_PER_PAGE, useMailboxes, useRecordThreads } from '../api'
import { EMAILS_SEND_PERMISSION, EMAIL_THREAD_PARAM, MAILBOXES_PATH } from '../constants'
import type { EmailRecordRef } from '../types'
import { ComposerDialog, type ReplyContext } from './Composer'
import { ThreadList } from './ThreadList'
import { ThreadReader } from './ThreadReader'

export interface EmailsTabProps {
  recordType: string
  recordId: string
  readOnly?: boolean
}

export function EmailsTab({ recordType, recordId }: EmailsTabProps) {
  const { t } = useTranslation('email')
  const { hasPermission } = useSession()
  const [searchParams, setSearchParams] = useSearchParams()
  const [page, setPage] = useState(1)
  const [composer, setComposer] = useState<{ reply: ReplyContext | null } | null>(null)
  const record: EmailRecordRef = { record_type: recordType, record_id: recordId }
  const canSend = hasPermission(EMAILS_SEND_PERMISSION)
  const mailboxes = useMailboxes()
  const threads = useRecordThreads(record, page)
  const threadId = searchParams.get(EMAIL_THREAD_PARAM)
  const ownMailboxes = mailboxes.data ?? []
  const hasMailbox = ownMailboxes.length > 0

  const openThread = (id: string | null) =>
    setSearchParams(
      (current) => {
        const next = new URLSearchParams(current)
        if (id) next.set(EMAIL_THREAD_PARAM, id)
        else next.delete(EMAIL_THREAD_PARAM)
        return next
      },
      { replace: false },
    )

  const composerDialog = (
    <ComposerDialog
      open={composer !== null}
      onOpenChange={(open) => !open && setComposer(null)}
      record={record}
      mailboxes={ownMailboxes}
      reply={composer?.reply ?? null}
    />
  )

  if (threadId) {
    return (
      <>
        <ThreadReader
          threadId={threadId}
          canSend={canSend && hasMailbox}
          onBack={() => openThread(null)}
          onReply={(reply) => setComposer({ reply })}
        />
        {composerDialog}
      </>
    )
  }

  return (
    <div className="grid gap-4">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">{t('tab.description')}</p>
        {canSend && hasMailbox && (
          <Button onClick={() => setComposer({ reply: null })} className="w-full sm:w-auto">
            <MailPlus aria-hidden="true" />
            {t('tab.compose')}
          </Button>
        )}
      </div>
      {mailboxes.isSuccess && !hasMailbox && <ConnectMailboxCallout />}
      {threads.isPending && (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      )}
      {threads.isError && <ErrorState onRetry={() => void threads.refetch()} />}
      {threads.isSuccess && threads.data.data.length === 0 && (
        <EmptyState
          icon={Inbox}
          title={t('tab.emptyTitle')}
          description={hasMailbox ? t('tab.emptyDescription') : t('tab.emptyNoMailbox')}
        />
      )}
      {threads.isSuccess && threads.data.data.length > 0 && (
        <>
          <PaginationSummary page={threads.data.meta.page} perPage={threads.data.meta.per_page || THREADS_PER_PAGE} total={threads.data.meta.total} />
          <ThreadList threads={threads.data.data} onOpen={(thread) => openThread(thread.id)} />
          <Pagination
            page={threads.data.meta.page}
            perPage={threads.data.meta.per_page || THREADS_PER_PAGE}
            total={threads.data.meta.total}
            onPageChange={setPage}
          />
        </>
      )}
      {composerDialog}
    </div>
  )
}

function ConnectMailboxCallout() {
  const { t } = useTranslation('email')
  return (
    <div className="flex flex-col gap-3 rounded-lg border border-dashed p-4 sm:flex-row sm:items-center sm:justify-between">
      <div className="flex items-start gap-3">
        <Plug className="mt-0.5 size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
        <div className="grid gap-0.5">
          <p className="text-sm font-medium">{t('tab.noMailboxTitle')}</p>
          <p className="text-sm text-muted-foreground">{t('tab.noMailboxDescription')}</p>
        </div>
      </div>
      <Button variant="outline" asChild className="w-full sm:w-auto">
        <Link to={MAILBOXES_PATH}>{t('tab.connectMailbox')}</Link>
      </Button>
    </div>
  )
}
