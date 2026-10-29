import type { CreateActionRenderProps } from '@/kernel/modules/define'
import { ActivityFormDialog } from './components/ActivityFormDialog'

export function CreateTaskDialog({ open, target, onClose }: CreateActionRenderProps) {
  if (!open) return null
  return <ActivityFormDialog open onOpenChange={(next) => !next && onClose()} target={target} mode="task" />
}
