import { CalendarClock, NotebookPen, Paperclip, RotateCcw, SquareCheck, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { AttachmentsPanel, RecordDetailPage, type RecordItem } from '@/sdk'
import { useDeleteActivity, useToggleActivity } from './api'
import { ActivityScheduleDialog, type ScheduledActivity } from './components/ActivityScheduleDialog'
import { activityErrorKey } from './lib/errors'
import { ACTIVITY_RECORD_TYPE, TASK_DETAIL_FIELDS, TASKS_PATH } from './resource'

function TaskActions({ record }: { record: RecordItem }) {
  const { t } = useTranslation('tasks')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const toggle = useToggleActivity()
  const remove = useDeleteActivity()
  const [confirming, setConfirming] = useState(false)
  const [booking, setBooking] = useState(false)
  const done = record.status === 'done'
  const title = typeof record.title === 'string' ? record.title : ''

  const onToggle = () =>
    toggle.mutate(
      { id: record.id, done: !done },
      {
        onSuccess: () =>
          toast({ title: t(done ? 'tab.reopenedToast' : 'tab.completedToast', { title }), variant: 'success' }),
        onError: (error) => toast({ title: t(activityErrorKey(error)), variant: 'destructive' }),
      },
    )

  return (
    <div className="flex flex-wrap gap-2">
      {hasPermission('tasks.update') && record.status !== 'canceled' && (
        <Button size="sm" variant={done ? 'outline' : 'default'} onClick={onToggle} disabled={toggle.isPending}>
          {done ? <RotateCcw aria-hidden="true" /> : <SquareCheck aria-hidden="true" />}
          {t(done ? 'detail.reopen' : 'detail.complete')}
        </Button>
      )}
      {hasPermission('tasks.update') && record.status === 'open' && (
        <Button size="sm" variant="outline" onClick={() => setBooking(true)}>
          <CalendarClock aria-hidden="true" />
          {t('schedule.open')}
        </Button>
      )}
      {hasPermission('tasks.delete') && (
        <Button size="sm" variant="outline" onClick={() => setConfirming(true)}>
          <Trash2 aria-hidden="true" />
          {t('detail.delete')}
        </Button>
      )}
      <ActivityScheduleDialog
        activity={record as unknown as ScheduledActivity}
        open={booking}
        onOpenChange={setBooking}
      />
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('detail.deleteTitle')}
        description={t('detail.deleteDescription', { title })}
        confirmLabel={t('detail.delete')}
        variant="destructive"
        pending={remove.isPending}
        onConfirm={async () => {
          await remove.mutateAsync(record.id)
          toast({ title: t('detail.deleted'), variant: 'success' })
          navigate(TASKS_PATH)
        }}
      />
    </div>
  )
}

export function TaskDetailPage() {
  const { t } = useTranslation('tasks')
  const { id = '' } = useParams()
  return (
    <RecordDetailPage
      recordType={ACTIVITY_RECORD_TYPE}
      recordId={id}
      fieldKeys={TASK_DETAIL_FIELDS}
      readOnlyFieldKeys={['id', 'created_at', 'updated_at', 'created_by', 'completed_at', 'status']}
      back={{ to: TASKS_PATH, label: t('detail.back') }}
      actions={(record) => <TaskActions record={record} />}
      tabs={[
        {
          key: 'notes',
          group: 'activity',
          label: t('detail.notesTab'),
          icon: NotebookPen,
          content: null,
        },
        {
          key: 'files',
          group: 'activity',
          label: t('detail.filesTab'),
          icon: Paperclip,
          content: <AttachmentsPanel recordType={ACTIVITY_RECORD_TYPE} recordId={id} />,
        },
      ]}
    />
  )
}
