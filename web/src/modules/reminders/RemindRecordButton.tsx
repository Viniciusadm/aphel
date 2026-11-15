import { AlarmClock } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { useRecordReminders } from './api'
import { ReminderDialog } from './ReminderDialog'

export interface RemindRecordButtonProps {
  recordType: string
  recordId: string
  title?: string | null
}

export function RemindRecordButton({ recordType, recordId, title }: RemindRecordButtonProps) {
  const { t } = useTranslation('reminders')
  const [open, setOpen] = useState(false)
  const reminders = useRecordReminders(recordType, recordId)
  if (reminders.isError) return null
  const count = reminders.data?.meta.total ?? 0
  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        <AlarmClock aria-hidden="true" />
        {count > 0 ? t('button.withCount', { count }) : t('button.remind')}
      </Button>
      <ReminderDialog open={open} onOpenChange={setOpen} target={{ recordType, recordId, title }} />
    </>
  )
}
