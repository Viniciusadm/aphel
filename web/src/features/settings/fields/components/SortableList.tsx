import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type Announcements,
  type DragEndEvent,
  type UniqueIdentifier,
} from '@dnd-kit/core'
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { moveId } from '../lib/order'

interface SortableListProps<T> {
  items: T[]
  getId: (item: T) => string
  getLabel: (item: T) => string
  onReorder: (ids: string[], from: number, to: number) => void
  renderItem: (item: T, handle: ReactNode, index: number) => ReactNode
  disabled?: boolean
  label?: string
  className?: string
}

export function SortableList<T>({
  items,
  getId,
  getLabel,
  onReorder,
  renderItem,
  disabled = false,
  label,
  className,
}: SortableListProps<T>) {
  const { t } = useTranslation('fields')
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )
  const ids = items.map(getId)
  const labelOf = (id: UniqueIdentifier) => {
    const item = items.find((candidate) => getId(candidate) === String(id))
    return item ? getLabel(item) : String(id)
  }
  const positionOf = (id: UniqueIdentifier) => ids.indexOf(String(id)) + 1

  const announcements: Announcements = {
    onDragStart: ({ active }) => t('reorder.picked', { label: labelOf(active.id) }),
    onDragOver: ({ active, over }) =>
      over
        ? t('reorder.over', { label: labelOf(active.id), position: positionOf(over.id), total: ids.length })
        : undefined,
    onDragEnd: ({ active, over }) =>
      over
        ? t('reorder.dropped', { label: labelOf(active.id), position: positionOf(over.id), total: ids.length })
        : undefined,
    onDragCancel: ({ active }) => t('reorder.cancelled', { label: labelOf(active.id) }),
  }

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return
    const from = ids.indexOf(String(active.id))
    const to = ids.indexOf(String(over.id))
    if (from < 0 || to < 0) return
    onReorder(moveId(ids, from, to), from, to)
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
      accessibility={{ announcements, screenReaderInstructions: { draggable: t('reorder.instructions') } }}
    >
      <SortableContext items={ids} strategy={verticalListSortingStrategy} disabled={disabled}>
        <ul className={cn('grid gap-2', className)} aria-label={label}>
          {items.map((item, index) => (
            <SortableItem key={ids[index]} id={ids[index] ?? ''} label={getLabel(item)} disabled={disabled}>
              {(handle) => renderItem(item, handle, index)}
            </SortableItem>
          ))}
        </ul>
      </SortableContext>
    </DndContext>
  )
}

interface SortableItemProps {
  id: string
  label: string
  disabled: boolean
  children: (handle: ReactNode) => ReactNode
}

function SortableItem({ id, label, disabled, children }: SortableItemProps) {
  const { t } = useTranslation('fields')
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({
    id,
    disabled,
  })
  const handle = (
    <button
      type="button"
      ref={setActivatorNodeRef}
      className="flex size-8 shrink-0 cursor-grab touch-none items-center justify-center rounded-md text-muted-foreground transition-colors outline-none hover:bg-accent hover:text-accent-foreground focus-visible:ring-2 focus-visible:ring-ring active:cursor-grabbing disabled:cursor-not-allowed disabled:opacity-50"
      disabled={disabled}
      {...attributes}
      {...listeners}
      aria-label={t('reorder.handle', { label })}
    >
      <GripVertical className="size-4" aria-hidden />
    </button>
  )
  return (
    <li
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn('relative', isDragging && 'z-10 opacity-90 shadow-lg')}
    >
      {children(handle)}
    </li>
  )
}
