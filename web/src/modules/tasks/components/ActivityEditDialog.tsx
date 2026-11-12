import type { Activity } from '@/sdk'
import { ActivityFormDialog } from './ActivityFormDialog'

export function ActivityEditDialog({
  record,
  open,
  onOpenChange,
}: {
  record: unknown
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  return <ActivityFormDialog open={open} onOpenChange={onOpenChange} activity={record as Activity} mode="task" />
}
