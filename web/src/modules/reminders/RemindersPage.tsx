import { AlarmClock, Check, Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Pagination } from '@/components/ui/pagination'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, recordDetailHref, useRecordFormatting } from '@/sdk'
import { REMINDERS_PER_PAGE, useDeleteReminder, useReminders, useUpdateReminder } from './api'
import { useRecordTypeLabel } from './labels'
import { ReminderDialog } from './ReminderDialog'
import type { Reminder, ReminderStatus } from './types'

const STATUSES: ReminderStatus[] = ['pending', 'sent', 'dismissed']

export function RemindersPage() {
  const { t } = useTranslation('reminders')
  const [status, setStatus] = useState<ReminderStatus>('pending')
  const [page, setPage] = useState(1)
  const [creating, setCreating] = useState(false)
  const [editing, setEditing] = useState<Reminder | null>(null)
  const [removing, setRemoving] = useState<Reminder | null>(null)
  const reminders = useReminders(status, page)
  const dismiss = useUpdateReminder()
  const remove = useDeleteReminder()

  let body
  if (reminders.isPending) {
    body = <Skeleton className="h-40 w-full" aria-busy="true" />
  } else if (reminders.isError) {
    body = <ErrorState onRetry={() => void reminders.refetch()} />
  } else if (reminders.data.data.length === 0) {
    body = (
      <EmptyState
        icon={AlarmClock}
        title={t(`empty.${status}.title`)}
        description={t(`empty.${status}.description`)}
        action={
          status === 'pending' ? (
            <Button size="sm" onClick={() => setCreating(true)}>
              <Plus aria-hidden="true" />
              {t('page.new')}
            </Button>
          ) : undefined
        }
      />
    )
  } else {
    body = (
      <>
        <ul className="divide-y rounded-lg border bg-card" aria-label={t(`status.${status}`)}>
          {reminders.data.data.map((reminder) => (
            <ReminderRow
              key={reminder.id}
              reminder={reminder}
              onEdit={() => setEditing(reminder)}
              onDismiss={() =>
                dismiss.mutate(
                  { id: reminder.id, body: { status: 'dismissed' } },
                  { onSuccess: () => notifySuccess(t('toast.dismissed')), onError: (error) => notifyError(error) },
                )
              }
              onRemove={() => setRemoving(reminder)}
              busy={dismiss.isPending}
            />
          ))}
        </ul>
        <Pagination page={page} perPage={REMINDERS_PER_PAGE} total={reminders.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <Page>
      <PageHeader
        title={t('page.title')}
        description={t('page.description')}
        actions={
          <Button onClick={() => setCreating(true)}>
            <Plus aria-hidden="true" />
            {t('page.new')}
          </Button>
        }
      />
      <SegmentedControl
        aria-label={t('page.statusFilter')}
        value={status}
        onValueChange={(value) => {
          setStatus(value)
          setPage(1)
        }}
        options={STATUSES.map((value) => ({ value, label: t(`status.${value}`) }))}
        className="w-fit"
      />
      {body}
      <ReminderDialog open={creating} onOpenChange={setCreating} />
      <ReminderDialog open={editing !== null} onOpenChange={(open) => !open && setEditing(null)} reminder={editing ?? undefined} />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={t('remove.title')}
        description={t('remove.description')}
        confirmLabel={t('remove.confirm')}
        variant="destructive"
        pending={remove.isPending}
        onConfirm={() => {
          if (removing) remove.mutate(removing.id, { onSuccess: () => notifySuccess(t('toast.removed')), onError: (error) => notifyError(error) })
        }}
      />
    </Page>
  )
}

interface ReminderRowProps {
  reminder: Reminder
  busy: boolean
  onEdit: () => void
  onDismiss: () => void
  onRemove: () => void
}

function ReminderRow({ reminder, busy, onEdit, onDismiss, onRemove }: ReminderRowProps) {
  const { t } = useTranslation('reminders')
  const formatting = useRecordFormatting()
  const typeLabel = useRecordTypeLabel()
  const href = reminder.record ? recordDetailHref(reminder.record.record_type, reminder.record.record_id) : undefined
  const recordTitle = reminder.record ? (reminder.record.title ?? t('page.untitled')) : null
  const pending = reminder.status === 'pending'
  return (
    <li className="flex flex-wrap items-center justify-between gap-3 p-4">
      <div className="grid min-w-0 flex-1 gap-1">
        <span className="break-words font-medium">{reminder.note ?? recordTitle ?? t('page.noNote')}</span>
        <span className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
          <span>{formatDateTime(reminder.remind_at, formatting)}</span>
          {reminder.record && recordTitle && (
            <span className="min-w-0 truncate">
              {typeLabel(reminder.record.record_type)}:{' '}
              {href ? (
                <Link to={href} className="text-primary underline-offset-4 hover:underline">
                  {recordTitle}
                </Link>
              ) : (
                recordTitle
              )}
            </span>
          )}
        </span>
      </div>
      <div className="flex items-center gap-1.5">
        {!pending && <Badge tone={reminder.status === 'sent' ? 'neutral' : 'primary'}>{t(`status.${reminder.status}`)}</Badge>}
        {pending && (
          <>
            <Button size="sm" variant="outline" disabled={busy} onClick={onDismiss}>
              <Check aria-hidden="true" />
              {t('actions.dismiss')}
            </Button>
            <Button size="icon" variant="ghost" className="size-8" aria-label={t('actions.edit')} onClick={onEdit}>
              <Pencil aria-hidden="true" />
            </Button>
          </>
        )}
        <Button size="icon" variant="ghost" className="size-8 text-destructive" aria-label={t('actions.remove')} onClick={onRemove}>
          <Trash2 aria-hidden="true" />
        </Button>
      </div>
    </li>
  )
}
