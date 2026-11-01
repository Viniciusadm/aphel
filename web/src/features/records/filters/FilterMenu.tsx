import { ListFilter } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMediaQuery } from '../calendar/useMediaQuery'
import { Dialog, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from '@/components/ui/dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import type { FieldSchema, FilterNode } from '../types'
import { useFilterSchema } from './extensions'
import { FilterBuilder } from './FilterBuilder'
import { countConditions, emptyCondition, serializeFilter, toDraft, type DraftGroup } from './model'

export interface FilterMenuProps {
  schema: FieldSchema
  filter: FilterNode | null
  onApply: (filter: FilterNode | null) => void
}

function initialDraft(filter: FilterNode | null): DraftGroup {
  const draft = toDraft(filter)
  return draft.children.length === 0 ? { ...draft, children: [emptyCondition()] } : draft
}

export function FilterMenu({ schema: baseSchema, filter, onApply }: FilterMenuProps) {
  const { t } = useTranslation('records-table')
  const { t: common } = useTranslation()
  const mobile = useMediaQuery('(max-width: 767px)')
  const { schema, extensions } = useFilterSchema(baseSchema)
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState<DraftGroup>(() => initialDraft(filter))
  const active = countConditions(filter)

  const handleOpenChange = (next: boolean) => {
    if (next) setDraft(initialDraft(filter))
    setOpen(next)
  }

  const apply = (next: FilterNode | null) => {
    onApply(next)
    setOpen(false)
  }

  const trigger = (
    <Button variant="outline" size="sm" aria-label={active > 0 ? t('filters.active', { count: active }) : undefined}>
          <ListFilter aria-hidden />
          {t('filters.button')}
          {active > 0 && (
            <Badge variant="secondary" className="px-1.5">
              {active}
            </Badge>
          )}
        </Button>
  )
  const editor = (
    <form
          className="grid gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            apply(serializeFilter(draft, schema))
          }}
        >
          <FilterBuilder schema={schema} value={draft} onChange={setDraft} extensions={extensions} allowRelated />
          <p className="text-xs text-muted-foreground">{t('filters.incomplete')}</p>
          <div className="sticky bottom-0 flex flex-wrap justify-end gap-2 border-t bg-background pt-3">
            <Button type="button" variant="ghost" size="sm" onClick={() => mobile ? setDraft(initialDraft(null)) : apply(null)}>
              {t('filters.clear')}
            </Button>
            {mobile && <Button type="button" variant="outline" size="sm" onClick={() => setOpen(false)}>{common('actions.cancel')}</Button>}
            <Button type="submit" size="sm">
              {t('filters.apply')}
            </Button>
          </div>
        </form>
  )
  if (mobile) return <Dialog open={open} onOpenChange={handleOpenChange}>
    <DialogTrigger asChild>{trigger}</DialogTrigger>
    <DialogContent side="bottom" className="max-h-[94dvh]">
      <div className="grid min-w-0 gap-4 p-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
        <div className="grid gap-1 pr-10"><DialogTitle>{t('filters.title')}</DialogTitle><DialogDescription>{t('mobileControls.description')}</DialogDescription></div>
        {editor}
      </div>
    </DialogContent>
  </Dialog>
  return <Popover open={open} onOpenChange={handleOpenChange}>
    <PopoverTrigger asChild>{trigger}</PopoverTrigger>
    <PopoverContent align="start" aria-label={t('filters.title')} className="grid max-h-[min(36rem,80vh)] w-[44rem] gap-3 overflow-y-auto">{editor}</PopoverContent>
  </Popover>
}
