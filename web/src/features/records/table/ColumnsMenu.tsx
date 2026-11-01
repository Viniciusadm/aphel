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
import { Columns3, EyeOff, GripVertical, Plus } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import { activeFields } from '../fields'
import type { FieldDefinition, FieldSchema } from '../types'
import { moveKey, resolveColumnFields } from './columns'

export interface ColumnsMenuProps {
  schema: FieldSchema
  columns: string[]
  onChange: (columns: string[]) => void
  onReset?: () => void
  titleField?: string
  label?: string
}

export function ColumnsMenu({ schema, columns, onChange, onReset, titleField, label }: ColumnsMenuProps) {
  const { t } = useTranslation('records-table')
  const visible = useMemo(() => resolveColumnFields(schema, columns, titleField), [schema, columns, titleField])
  const hidden = useMemo(
    () => activeFields(schema).filter((field) => !visible.includes(field)),
    [schema, visible],
  )
  const keys = visible.map((field) => field.key)

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm">
          <Columns3 aria-hidden />
          {label ?? t('columns.button')}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="grid max-h-[min(32rem,80vh)] w-72 gap-3 overflow-y-auto" aria-label={label ?? t('columns.title')}>
        <div className="flex items-center justify-between gap-2">
          <p className="text-sm font-medium">{label ?? t('columns.title')}</p>
          {onReset && (
            <Button variant="link" size="sm" className="h-auto px-0 text-xs" onClick={onReset}>
              {t('columns.reset')}
            </Button>
          )}
        </div>
        <section className="grid gap-1">
          <p className="text-xs font-medium text-muted-foreground">{t('columns.visible')}</p>
          <SortableColumns
            fields={visible}
            onReorder={(from, to) => onChange(moveKey(keys, from, to))}
            onHide={(key) => onChange(keys.filter((candidate) => candidate !== key))}
          />
        </section>
        <section className="grid gap-1">
          <p className="text-xs font-medium text-muted-foreground">{t('columns.hidden')}</p>
          {hidden.length === 0 ? (
            <p className="text-xs text-muted-foreground">{t('columns.noHidden')}</p>
          ) : (
            <ul className="grid gap-0.5">
              {hidden.map((field) => (
                <li key={field.key}>
                  <button
                    type="button"
                    className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm outline-none hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent"
                    aria-label={t('columns.show', { label: field.label })}
                    onClick={() => onChange([...keys, field.key])}
                  >
                    <Plus className="size-4 text-muted-foreground" aria-hidden />
                    <span className="truncate">{field.label}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </PopoverContent>
    </Popover>
  )
}

interface SortableColumnsProps {
  fields: FieldDefinition[]
  onReorder: (from: number, to: number) => void
  onHide: (key: string) => void
}

function SortableColumns({ fields, onReorder, onHide }: SortableColumnsProps) {
  const { t } = useTranslation('records-table')
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )
  const ids = fields.map((field) => field.key)
  const labelOf = (id: UniqueIdentifier) => fields.find((field) => field.key === String(id))?.label ?? String(id)
  const positionOf = (id: UniqueIdentifier) => ids.indexOf(String(id)) + 1

  const announcements: Announcements = {
    onDragStart: ({ active }) => t('columns.picked', { label: labelOf(active.id) }),
    onDragOver: ({ active, over }) =>
      over ? t('columns.over', { label: labelOf(active.id), position: positionOf(over.id), total: ids.length }) : undefined,
    onDragEnd: ({ active, over }) =>
      over
        ? t('columns.dropped', { label: labelOf(active.id), position: positionOf(over.id), total: ids.length })
        : undefined,
    onDragCancel: ({ active }) => t('columns.cancelled', { label: labelOf(active.id) }),
  }

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return
    const from = ids.indexOf(String(active.id))
    const to = ids.indexOf(String(over.id))
    if (from >= 0 && to >= 0) onReorder(from, to)
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
      accessibility={{ announcements, screenReaderInstructions: { draggable: t('columns.instructions') } }}
    >
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        <ul className="grid gap-0.5" aria-label={t('columns.visible')}>
          {fields.map((field) => (
            <SortableColumn
              key={field.key}
              field={field}
              canHide={fields.length > 1}
              onHide={() => onHide(field.key)}
            />
          ))}
        </ul>
      </SortableContext>
    </DndContext>
  )
}

interface SortableColumnProps {
  field: FieldDefinition
  canHide: boolean
  onHide: () => void
}

function SortableColumn({ field, canHide, onHide }: SortableColumnProps) {
  const { t } = useTranslation('records-table')
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({
    id: field.key,
  })
  return (
    <li
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn(
        'flex items-center gap-1 rounded-sm bg-popover pr-1 text-sm',
        isDragging && 'relative z-10 shadow-md',
      )}
    >
      <button
        type="button"
        ref={setActivatorNodeRef}
        className="flex size-7 shrink-0 cursor-grab touch-none items-center justify-center rounded-sm text-muted-foreground outline-none hover:bg-accent hover:text-accent-foreground focus-visible:ring-2 focus-visible:ring-ring active:cursor-grabbing"
        {...attributes}
        {...listeners}
        aria-label={t('columns.handle', { label: field.label })}
      >
        <GripVertical className="size-4" aria-hidden />
      </button>
      <span className="min-w-0 flex-1 truncate">{field.label}</span>
      <button
        type="button"
        disabled={!canHide}
        className="flex size-7 shrink-0 items-center justify-center rounded-sm text-muted-foreground outline-none hover:bg-accent hover:text-accent-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40"
        aria-label={t('columns.hide', { label: field.label })}
        onClick={onHide}
      >
        <EyeOff className="size-4" aria-hidden />
      </button>
    </li>
  )
}
