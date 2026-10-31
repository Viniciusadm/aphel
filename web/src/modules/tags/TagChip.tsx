import { X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

export interface TagChipProps {
  name: string
  color: string
  onRemove?: () => void
  disabled?: boolean
  className?: string
}

export function TagChip({ name, color, onRemove, disabled, className }: TagChipProps) {
  const { t } = useTranslation('tags')
  return (
    <Badge
      variant="outline"
      className={cn('max-w-full min-w-0 gap-1.5 rounded-full bg-background', className)}
      icon={<span aria-hidden="true" className="size-2 shrink-0 rounded-full" style={{ backgroundColor: color }} />}
    >
      <span className="truncate">{name}</span>
      {onRemove && (
        <button
          type="button"
          className="-mr-1 rounded-full p-0.5 text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
          aria-label={t('chips.remove', { name })}
          disabled={disabled}
          onClick={onRemove}
        >
          <X className="size-3" aria-hidden="true" />
        </button>
      )}
    </Badge>
  )
}

export interface TagChipListProps {
  tags: { id: string; name: string; color: string }[]
  max?: number
  className?: string
}

export function TagChipList({ tags, max = 3, className }: TagChipListProps) {
  const { t } = useTranslation('tags')
  if (tags.length === 0) return null
  const visible = tags.slice(0, max)
  const hidden = tags.length - visible.length
  return (
    <span className={cn('inline-flex min-w-0 flex-wrap items-center gap-1', className)}>
      {visible.map((tag) => (
        <TagChip key={tag.id} name={tag.name} color={tag.color} />
      ))}
      {hidden > 0 && (
        <span className="text-xs text-muted-foreground" title={tags.slice(max).map((tag) => tag.name).join(', ')}>
          {t('chips.more', { count: hidden })}
        </span>
      )}
    </span>
  )
}
