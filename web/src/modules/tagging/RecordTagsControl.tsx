import { Check, Tag as TagIcon } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import { isTaggable, tagUpdatePermission, useRecordTagMutation, useRecordTags, useTags } from './api'
import { TagChip } from './TagChip'

export interface RecordTagsControlProps {
  recordType: string
  recordId: string
  readOnly?: boolean
  className?: string
}

export function RecordTagsControl({ recordType, recordId, readOnly = false, className }: RecordTagsControlProps) {
  const { t } = useTranslation('tagging')
  const { hasPermission } = useSession()
  const permission = tagUpdatePermission(recordType)
  const canEdit = !readOnly && Boolean(permission && hasPermission(permission))
  const assigned = useRecordTags(recordType, recordId)
  const mutation = useRecordTagMutation(recordType, recordId)
  const [open, setOpen] = useState(false)

  if (!isTaggable(recordType)) return null
  const tags = assigned.data ?? []

  const toggle = (tagId: string, remove: boolean) => {
    mutation.mutate({ tagId, remove }, { onError: (error) => notifyError(error) })
  }

  return (
    <div className={cn('flex min-w-0 flex-wrap items-center gap-1.5', className)} aria-label={t('record.label')} role="group">
      {tags.map((tag) => (
        <TagChip
          key={tag.id}
          name={tag.name}
          color={tag.color}
          disabled={mutation.isPending}
          onRemove={canEdit ? () => toggle(tag.id, true) : undefined}
        />
      ))}
      {canEdit && (
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm">
              <TagIcon aria-hidden="true" />
              {t('record.manage')}
            </Button>
          </PopoverTrigger>
          <PopoverContent align="end" className="w-64 p-2">
            {open && (
              <TagPicker
                recordType={recordType}
                selectedIds={tags.map((tag) => tag.id)}
                disabled={mutation.isPending}
                onToggle={toggle}
              />
            )}
          </PopoverContent>
        </Popover>
      )}
    </div>
  )
}

interface TagPickerProps {
  recordType: string
  selectedIds: string[]
  disabled: boolean
  onToggle: (tagId: string, remove: boolean) => void
}

function TagPicker({ recordType, selectedIds, disabled, onToggle }: TagPickerProps) {
  const { t } = useTranslation('tagging')
  const available = useTags(recordType)
  const [search, setSearch] = useState('')
  const term = search.trim().toLocaleLowerCase()
  const items = (available.data ?? []).filter((tag) => !term || tag.name.toLocaleLowerCase().includes(term))

  return (
    <div className="grid gap-2">
      <Input
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        placeholder={t('record.search')}
        aria-label={t('record.search')}
        autoFocus
      />
      {available.isPending ? (
        <p className="px-1 text-sm text-muted-foreground">{t('record.loading')}</p>
      ) : items.length === 0 ? (
        <p className="px-1 text-sm text-muted-foreground">{t('record.empty')}</p>
      ) : (
        <ul className="grid max-h-64 gap-0.5 overflow-y-auto" aria-label={t('record.available')}>
          {items.map((tag) => {
            const selected = selectedIds.includes(tag.id)
            return (
              <li key={tag.id}>
                <button
                  type="button"
                  aria-pressed={selected}
                  disabled={disabled}
                  onClick={() => onToggle(tag.id, selected)}
                  className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
                >
                  <span aria-hidden="true" className="size-2.5 shrink-0 rounded-full" style={{ backgroundColor: tag.color }} />
                  <span className="min-w-0 flex-1 truncate">{tag.name}</span>
                  {selected && <Check className="size-4" aria-hidden="true" />}
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
