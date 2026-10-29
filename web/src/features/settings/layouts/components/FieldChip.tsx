import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { ArrowDown, ArrowUp, Eye, GripVertical, Lock, SlidersHorizontal, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { LayoutField } from '@/features/records/form/lib/layout'
import type { FieldDefinition } from '@/features/records/types'
import { cn } from '@/lib/utils'

export interface FieldChipProps {
  field: FieldDefinition
  rule?: LayoutField
  container: string
  locked?: boolean
  onMoveUp?: () => void
  onMoveDown?: () => void
  onRemove?: () => void
  onRules?: () => void
}

export function FieldChip({ field, rule, container, locked = false, onMoveUp, onMoveDown, onRemove, onRules }: FieldChipProps) {
  const { t } = useTranslation('layouts')
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({
    id: field.key,
    data: { container },
  })
  const placed = Boolean(rule)

  return (
    <li
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      data-field={field.key}
      className={cn(
        'flex min-w-0 items-center gap-1 rounded-md border bg-background px-1.5 py-1 text-sm shadow-xs',
        isDragging && 'relative z-10 opacity-80 ring-2 ring-ring',
      )}
    >
      <button
        type="button"
        ref={setActivatorNodeRef}
        className="flex size-7 shrink-0 cursor-grab touch-none items-center justify-center rounded text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none active:cursor-grabbing"
        aria-label={t('fields.drag', { field: field.label })}
        {...attributes}
        {...listeners}
      >
        <GripVertical className="size-4" aria-hidden="true" />
      </button>
      <span className="min-w-0 flex-1 truncate" title={field.label}>
        {field.label}
      </span>
      <span className="flex shrink-0 items-center gap-1">
        {field.is_required && (
          <Badge variant="secondary" className="hidden sm:inline-flex">
            {t('fields.required')}
          </Badge>
        )}
        {rule?.visible_when && (
          <Eye className="size-4 text-muted-foreground" aria-label={t('fields.visibleRule')} role="img" />
        )}
        {rule?.required_when && (
          <Badge variant="outline" className="hidden sm:inline-flex">
            {t('fields.requiredRule')}
          </Badge>
        )}
        {locked && <Lock className="size-4 text-muted-foreground" aria-label={t('fields.locked')} role="img" />}
      </span>
      {placed && (
        <span className="flex shrink-0 items-center">
          {onRules && (
            <Button type="button" variant="ghost" size="icon" className="size-7" aria-label={t('fields.rules', { field: field.label })} onClick={onRules}>
              <SlidersHorizontal aria-hidden="true" />
            </Button>
          )}
          {onMoveUp && (
            <Button type="button" variant="ghost" size="icon" className="size-7" aria-label={t('fields.moveUp', { field: field.label })} onClick={onMoveUp}>
              <ArrowUp aria-hidden="true" />
            </Button>
          )}
          {onMoveDown && (
            <Button type="button" variant="ghost" size="icon" className="size-7" aria-label={t('fields.moveDown', { field: field.label })} onClick={onMoveDown}>
              <ArrowDown aria-hidden="true" />
            </Button>
          )}
          {onRemove && !locked && (
            <Button type="button" variant="ghost" size="icon" className="size-7" aria-label={t('fields.remove', { field: field.label })} onClick={onRemove}>
              <X aria-hidden="true" />
            </Button>
          )}
        </span>
      )}
    </li>
  )
}
