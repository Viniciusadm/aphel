import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { DateTimeFieldInput } from '@/components/fields/DateTimeFieldInput'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { toast } from '@/components/ui/toast-store'
import { useRecordFormatting } from '@/features/records/formatting'
import { useUpdateActivity, type Activity, type RecurrenceApplyTo, type UpdateActivityRequest } from '../api'
import { activityErrorKey } from '../lib/errors'
import {
  buildRecurrenceRule,
  describeRecurrence,
  recurrenceError,
  recurrenceFromActivity,
  sameRecurrence,
  type RecurrenceFormState,
} from '../lib/recurrence'
import { RecurrenceFields } from './RecurrenceFields'
import { ReminderSelect } from './ReminderSelect'
import { Field } from '@/components/ui/field'

export type ScheduledActivity = Pick<
  Activity,
  'id' | 'title' | 'updated_at' | 'due_at' | 'reminder_minutes_before' | 'recurrence' | 'status'
>

export interface ActivityScheduleDialogProps {
  activity: ScheduledActivity
  open: boolean
  onOpenChange: (open: boolean) => void
}

const APPLY_TO_OPTIONS: RecurrenceApplyTo[] = ['this', 'following']

export function ActivityScheduleDialog({ activity, open, onOpenChange }: ActivityScheduleDialogProps) {
  const { t } = useTranslation('tasks')
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('schedule.title')}</DialogTitle>
          <DialogDescription>{t('schedule.description', { title: activity.title })}</DialogDescription>
        </DialogHeader>
        {open && <ScheduleForm activity={activity} onClose={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function ScheduleForm({ activity, onClose }: { activity: ScheduledActivity; onClose: () => void }) {
  const { t } = useTranslation('tasks')
  const formatting = useRecordFormatting()
  const update = useUpdateActivity()
  const baseId = useId()
  const recurring = Boolean(activity.recurrence && !activity.recurrence.ended)
  const initialRecurrence = recurrenceFromActivity(activity.recurrence)
  const [dueAt, setDueAt] = useState<string | null>(activity.due_at ?? null)
  const [reminder, setReminder] = useState<number | null>(activity.reminder_minutes_before ?? null)
  const [recurrence, setRecurrence] = useState<RecurrenceFormState>(initialRecurrence)
  const [applyTo, setApplyTo] = useState<RecurrenceApplyTo>('this')
  const [submitted, setSubmitted] = useState(false)

  const errorKey = recurrenceError(recurrence, dueAt)

  const send = async (request: UpdateActivityRequest, successKey: 'schedule.saved' | 'schedule.ended') => {
    try {
      await update.mutateAsync({ activity, request })
      toast({ title: t(successKey), variant: 'success' })
      onClose()
    } catch (error) {
      toast({ title: t(activityErrorKey(error)), variant: 'destructive' })
    }
  }

  const save = () => {
    setSubmitted(true)
    if (errorKey) return
    const request: UpdateActivityRequest = {}
    if ((activity.due_at ?? null) !== dueAt) request.due_at = dueAt
    if ((activity.reminder_minutes_before ?? null) !== reminder) request.reminder_minutes_before = reminder
    if (!sameRecurrence(recurrence, initialRecurrence)) {
      const rule = buildRecurrenceRule(recurrence)
      if (rule) request.recurrence = rule
      else if (recurring) request.recurrence = null
    }
    if (Object.keys(request).length === 0) {
      onClose()
      return
    }
    if (recurring) request.apply_to = applyTo
    void send(request, 'schedule.saved')
  }

  return (
    <>
      <div className="grid gap-4">
        {recurring && activity.recurrence && (
          <div className="grid gap-0.5 rounded-md border bg-muted/40 px-3 py-2 text-sm">
            <span className="font-medium">{describeRecurrence(activity.recurrence, t, formatting.locale)}</span>
            <span className="text-xs text-muted-foreground">
              {t('schedule.occurrence', { index: activity.recurrence.index })}
            </span>
          </div>
        )}
        <Field label={t('form.dueAt')}>
          {(fieldControl) => <DateTimeFieldInput
            id={fieldControl.id}
            value={dueAt}
            onChange={setDueAt}
            timeZone={formatting.timezone}
            locale={formatting.locale}
          />}
        </Field>
        <Field label={t('reminder.label')}>
          {(fieldControl) => <ReminderSelect id={fieldControl.id} value={reminder} onChange={setReminder} />}
        </Field>
        <RecurrenceFields
          value={recurrence}
          onChange={setRecurrence}
          error={submitted && errorKey ? t(errorKey) : null}
        />
        {recurring && (
          <fieldset className="grid gap-2">
            <legend className="mb-1 text-sm font-medium">{t('schedule.applyTo')}</legend>
            {APPLY_TO_OPTIONS.map((option) => (
              <label key={option} className="flex items-start gap-2 text-sm">
                <input
                  type="radio"
                  name={`${baseId}-apply`}
                  className="mt-0.5 size-4 accent-primary"
                  value={option}
                  checked={applyTo === option}
                  onChange={() => setApplyTo(option)}
                />
                <span className="grid gap-0.5">
                  <span>{t(`schedule.applyToOptions.${option}`)}</span>
                  <span className="text-xs text-muted-foreground">{t(`schedule.applyToHints.${option}`)}</span>
                </span>
              </label>
            ))}
          </fieldset>
        )}
      </div>
      <DialogFooter className="gap-2">
        {recurring && (
          <Button
            variant="outline"
            className="sm:mr-auto"
            disabled={update.isPending}
            onClick={() => void send({ recurrence: null }, 'schedule.ended')}
          >
            {t('schedule.endRecurrence')}
          </Button>
        )}
        <Button variant="outline" onClick={() => onClose()} disabled={update.isPending}>
          {t('form.cancel')}
        </Button>
        <Button onClick={save} disabled={update.isPending}>
          {t('schedule.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
