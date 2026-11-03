import { ActivityFormDialog } from './components/ActivityFormDialog'
import type { SlotProps } from '@/kernel/modules/slots'

export function CalendarCreateDialog({ open, dueAt, durationMinutes, onOpenChange }: SlotProps<'calendar.create'>) {
  return (
    <ActivityFormDialog
      open={open}
      mode="task"
      defaults={{ dueAt, durationMinutes }}
      onOpenChange={onOpenChange}
    />
  )
}
