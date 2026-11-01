import { ListFilter } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { useMediaQuery } from '../calendar/useMediaQuery'
import type { TimelineKindOption } from './types'

export function TimelineFilters({ options, value, onChange }: {
  options: TimelineKindOption[]
  value: string[]
  onChange: (value: string[]) => void
}) {
  const { t } = useTranslation('records-detail')
  const { t: common } = useTranslation()
  const mobile = useMediaQuery('(max-width: 767px)')
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState<string[]>([])
  const all = value.length === 0

  return <>
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <Button variant="outline" onClick={() => { setDraft(value); setOpen(true) }}>
        <ListFilter aria-hidden />{t('timeline.filterLabel')}
        {!all && <span className="rounded-full bg-primary/10 px-2 text-xs tabular-nums">{value.length}</span>}
      </Button>
      <span role="status" className="min-w-0 text-sm text-muted-foreground">
        {all ? t('timeline.allTypes') : options.filter((option) => value.includes(option.kind)).map((option) => option.label).join(', ')}
      </span>
      {!all && <Button variant="ghost" size="sm" onClick={() => onChange([])}>{t('timeline.clearFilters')}</Button>}
    </div>
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent side={mobile ? 'bottom' : 'center'} className="[--dialog-padding:0px]">
        <div className="grid min-h-0 gap-4 p-4 pb-[max(1rem,env(safe-area-inset-bottom))] sm:p-6">
          <div className="grid gap-1 pr-10">
            <DialogTitle>{t('timeline.filterLabel')}</DialogTitle>
            <DialogDescription>{t('timeline.filterDescription')}</DialogDescription>
          </div>
          <fieldset className="grid gap-1">
            <legend className="sr-only">{t('timeline.filterLabel')}</legend>
            <label className="flex min-h-12 cursor-pointer items-center gap-3 rounded-md px-2 hover:bg-muted">
              <Checkbox checked={draft.length === 0} onCheckedChange={() => setDraft([])} />{t('timeline.allTypes')}
            </label>
            {options.map((option) => <label key={option.kind} className="flex min-h-12 cursor-pointer items-center gap-3 rounded-md px-2 hover:bg-muted">
              <Checkbox checked={draft.includes(option.kind)} onCheckedChange={(checked) => setDraft((current) => checked ? [...current.filter((kind) => kind !== option.kind), option.kind] : current.filter((kind) => kind !== option.kind))} />
              <span className="text-sm">{option.label}</span>
            </label>)}
          </fieldset>
          <div className="flex flex-wrap justify-end gap-2 border-t pt-4">
            <Button variant="outline" onClick={() => setOpen(false)}>{common('actions.cancel')}</Button>
            <Button onClick={() => { onChange(draft); setOpen(false) }}>{t('timeline.applyFilters')}</Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  </>
}
