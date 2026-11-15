import { CalendarClock, CalendarPlus, RotateCcw, SquareCheck, Trash2, UserRoundPen } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { RecordActionItem, RecordActionsDivider, RecordDetailPage, type RecordItem } from '@/sdk'
import { TASK_DETAIL_FIELDS } from './resource'
import { useDeleteActivity, useNotifyDeleted, type Activity } from '@/sdk'
import { ActivityScheduleDialog, type ScheduledActivity } from './components/ActivityScheduleDialog'
import { ReassignDialog } from './components/ReassignDialog'
import { useActivityActions } from './lib/useActivityActions'
import { ACTIVITY_RECORD_TYPE, TASKS_PATH } from '@/sdk'

type TaskDialog = 'reassign' | 'schedule' | 'delete'

interface TaskDialogState {
  dialog: TaskDialog | null
  onDialog: (dialog: TaskDialog | null) => void
}

function TaskHeaderAction({ record, dialog, onDialog }: { record: RecordItem } & TaskDialogState) {
  const { t } = useTranslation('tasks')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const activity = record as unknown as Activity
  const actions = useActivityActions(activity)
  const remove = useDeleteActivity()
  const notifyDeleted = useNotifyDeleted()
  const done = activity.status === 'done'
  const title = activity.title

  return (
    <>
      {hasPermission('tasks.update') && activity.status !== 'canceled' && (
        <Button
          size="sm"
          variant={done ? 'outline' : 'default'}
          disabled={actions.pending}
          onClick={() => actions.setDone(!done)}
        >
          {done ? <RotateCcw aria-hidden="true" /> : <SquareCheck aria-hidden="true" />}
          {t(done ? 'detail.reopen' : 'detail.complete')}
        </Button>
      )}
      <ActivityScheduleDialog
        activity={record as unknown as ScheduledActivity}
        open={dialog === 'schedule'}
        onOpenChange={(open) => onDialog(open ? 'schedule' : null)}
      />
      <ReassignDialog
        item={dialog === 'reassign' ? activity : null}
        onOpenChange={(open) => !open && onDialog(null)}
        onConfirm={(_, ownerId) => {
          onDialog(null)
          void actions.reassign(ownerId)
        }}
      />
      <ConfirmDialog
        open={dialog === 'delete'}
        onOpenChange={(open) => !open && onDialog(null)}
        title={t('detail.deleteTitle')}
        description={t('detail.deleteDescription', { title })}
        confirmLabel={t('detail.delete')}
        variant="destructive"
        pending={remove.isPending}
        onConfirm={async () => {
          await remove.mutateAsync(activity.id)
          notifyDeleted({ recordType: 'tasks.task', recordId: activity.id, message: t('detail.deleted') })
          navigate(TASKS_PATH)
        }}
      />
    </>
  )
}

function TaskMenu({ record, onDialog }: { record: RecordItem } & Pick<TaskDialogState, 'onDialog'>) {
  const { t } = useTranslation('tasks')
  const { t: tl } = useTranslation('tasks', { keyPrefix: 'list' })
  const { hasPermission } = useSession()
  const activity = record as unknown as Activity
  const actions = useActivityActions(activity)
  const open = activity.status === 'open'
  const canUpdate = hasPermission('tasks.update')
  const canDelete = hasPermission('tasks.delete')

  return (
    <>
      {canUpdate && open && (
        <>
          <RecordActionItem icon={CalendarPlus} label={tl('actions.postponeDay')} onSelect={() => actions.postpone(1)} />
          <RecordActionItem icon={CalendarPlus} label={tl('actions.postponeWeek')} onSelect={() => actions.postpone(7)} />
          <RecordActionItem icon={UserRoundPen} label={tl('actions.reassign')} onSelect={() => onDialog('reassign')} />
          <RecordActionItem icon={CalendarClock} label={t('schedule.open')} onSelect={() => onDialog('schedule')} />
        </>
      )}
      {canDelete && (
        <>
          {canUpdate && open && <RecordActionsDivider />}
          <RecordActionItem icon={Trash2} label={t('detail.delete')} destructive onSelect={() => onDialog('delete')} />
        </>
      )}
    </>
  )
}

export function TaskDetailPage() {
  const { t } = useTranslation('tasks')
  const { id = '' } = useParams()
  const [dialog, setDialog] = useState<TaskDialog | null>(null)
  return (
    <RecordDetailPage
      recordType={ACTIVITY_RECORD_TYPE}
      recordId={id}
      fieldKeys={TASK_DETAIL_FIELDS}
      readOnlyFieldKeys={['id', 'created_at', 'updated_at', 'created_by', 'completed_at', 'status']}
      back={{ to: TASKS_PATH, label: t('detail.back') }}
      primaryAction={(record) => <TaskHeaderAction record={record} dialog={dialog} onDialog={setDialog} />}
      actions={(record) => <TaskMenu record={record} onDialog={setDialog} />}
      tabs={(defaults) => defaults.filter((tab) => tab.key === 'notes' || tab.key === 'files')}
    />
  )
}
