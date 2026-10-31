import { useState } from 'react'
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
import type { Activity } from '@/modules/activities/api'
import { useRecordFormatting } from '@/features/records/formatting'
import { Field } from '@/components/ui/field'

export interface RescheduleDialogProps {
  activity: Activity | null
  onOpenChange: (open: boolean) => void
  onConfirm: (activity: Activity, dueAt: string) => void
}

export function RescheduleDialog({ activity, onOpenChange, onConfirm }: RescheduleDialogProps) {
  const { t } = useTranslation('activities', { keyPrefix: 'calendar' })
  return (
    <Dialog open={activity !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('reschedule.title')}</DialogTitle>
          <DialogDescription>{t('reschedule.description', { title: activity?.title ?? '' })}</DialogDescription>
        </DialogHeader>
        {activity && (
          <RescheduleForm
            key={activity.id}
            activity={activity}
            onCancel={() => onOpenChange(false)}
            onConfirm={(dueAt) => onConfirm(activity, dueAt)}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

function RescheduleForm({
  activity,
  onCancel,
  onConfirm,
}: {
  activity: Activity
  onCancel: () => void
  onConfirm: (dueAt: string) => void
}) {
  const { t } = useTranslation('activities', { keyPrefix: 'calendar' })
  const formatting = useRecordFormatting()
  const [value, setValue] = useState<string | null>(activity.due_at ?? null)
  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (value) onConfirm(value)
      }}
    >
      <Field label={t('reschedule.dueAt')}>
        {(fieldControl) => <DateTimeFieldInput
          id={fieldControl.id}
          value={value}
          onChange={setValue}
          timeZone={formatting.timezone}
          locale={formatting.locale}
        />}
      </Field>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {t('reschedule.cancel')}
        </Button>
        <Button type="submit" disabled={!value}>
          {t('reschedule.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}
