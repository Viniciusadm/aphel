import { ListFilter, X } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { PageSizeSelect } from '@/components/ui/page-size-select'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from '@/components/ui/dialog'
import { NativeSelect } from '@/components/ui/native-select'
import { Slot } from '@/kernel/modules/Slot'
import { FilterBuilder } from '../filters/FilterBuilder'
import { useFilterSchema } from '../filters/extensions'
import { countConditions, serializeFilter, toDraft } from '../filters/model'
import type { ArchivedMode, FieldSchema, FilterNode, RecordViewConfig, SortSpec } from '../types'
import type { ListLayout } from '../calendar/layout'
import { ColumnsMenu } from './ColumnsMenu'
import { SearchBox } from './SearchBox'
import { MAX_SORTS } from './query'

export interface MobileListOptions {
  filter: FilterNode | null
  sort: SortSpec[]
  columns: string[]
  density: RecordViewConfig['density']
  archived: ArchivedMode
  layout: ListLayout
  perPage: number
  calendarField: string
}

interface MobileRecordToolbarProps {
  recordType: string
  schema: FieldSchema
  titleField: string
  resetColumns: string[]
  view: ReactNode
  search: string
  onSearch: (value: string) => void
  options: MobileListOptions
  allowArchived: boolean
  dateFields: { key: string; label: string }[]
  onApply: (value: MobileListOptions) => void
}

export function MobileRecordToolbar({ recordType, schema: baseSchema, titleField, resetColumns, view, search, onSearch, options, allowArchived, dateFields, onApply }: MobileRecordToolbarProps) {
  const { t } = useTranslation('records-table')
  const { t: calendar } = useTranslation('calendar')
  const { t: common } = useTranslation()
  const { schema, extensions } = useFilterSchema(baseSchema)
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState(options)
  const [conditions, setConditions] = useState(() => toDraft(options.filter))
  const active = countConditions(options.filter) + (allowArchived && options.archived !== 'exclude' ? 1 : 0)
  const sortable = schema.fields.filter((field) => field.is_active && field.is_sortable)
  const changeSort = (index: number, patch: Partial<SortSpec>) => setDraft((current) => ({ ...current, sort: current.sort.map((item, position) => position === index ? { ...item, ...patch } : item) }))

  const show = () => {
    setDraft(options)
    setConditions(toDraft(options.filter))
    setOpen(true)
  }

  return <Dialog open={open} onOpenChange={setOpen}>
    <div className="grid min-w-0 gap-3">
      <div className="flex min-w-0 flex-wrap items-center gap-2">{view}</div>
      <div className="flex min-w-0 items-center gap-2">
        <SearchBox value={search} onChange={onSearch} className="min-w-0 flex-1" />
        <DialogTrigger asChild><Button variant="outline" className="shrink-0" onClick={show} aria-label={t('mobileControls.title')}>
          <ListFilter aria-hidden />{t('filters.button')}{active > 0 && <span className="rounded-full bg-primary/10 px-1.5 text-xs tabular-nums">{active}</span>}
        </Button></DialogTrigger>
      </div>
      {(active > 0 || options.sort.length > 0) && <p className="text-xs text-muted-foreground" role="status">
        {[active > 0 ? t('mobileControls.filterCount', { count: active }) : '', options.sort.length > 0 ? t('mobileControls.sortCount', { count: options.sort.length }) : ''].filter(Boolean).join(' · ')}
      </p>}
    </div>
      <DialogContent side="bottom" className="max-h-[94dvh]">
        <form className="grid min-w-0 gap-6 p-4 pb-[max(1rem,env(safe-area-inset-bottom))]" onSubmit={(event) => {
          event.preventDefault()
          onApply({ ...draft, filter: serializeFilter(conditions, schema) })
          setOpen(false)
        }}>
          <div className="grid gap-1 pr-10"><DialogTitle>{t('mobileControls.title')}</DialogTitle><DialogDescription>{t('mobileControls.description')}</DialogDescription></div>
          <section className="grid min-w-0 gap-3">
            <h2 className="text-sm font-semibold">{t('filters.title')}</h2>
            <FilterBuilder schema={schema} value={conditions} onChange={setConditions} extensions={extensions} allowRelated />
            <p className="text-xs text-muted-foreground">{t('filters.incomplete')}</p>
            <Slot name="record.list.toolbar" props={{ recordType, filter: serializeFilter(conditions, schema), applyFilter: (filter) => setConditions(toDraft(filter)) }} />
            {allowArchived && <label className="grid gap-2 text-sm"><span>{t('archived.label')}</span><NativeSelect value={draft.archived} onValueChange={(value) => setDraft((current) => ({ ...current, archived: value as ArchivedMode }))}>
              <option value="exclude">{t('archived.exclude')}</option><option value="include">{t('archived.include')}</option><option value="only">{t('archived.only')}</option>
            </NativeSelect></label>}
          </section>
          <details className="min-w-0 border-t pt-4">
            <summary className="min-h-11 cursor-pointer text-sm font-semibold">{t('table.sortOptions')}{draft.sort.length > 0 ? ` (${draft.sort.length})` : ''}</summary>
            <div className="grid gap-3 pt-3">
              {draft.sort.map((item, index) => <div key={index} className="grid gap-2 rounded-lg border p-3">
                <label className="grid gap-1 text-sm"><span>{t('mobileControls.sortField', { number: index + 1 })}</span><NativeSelect value={item.field} onValueChange={(field) => changeSort(index, { field })}>
                  {!sortable.some((field) => field.key === item.field) && <option value={item.field}>{item.field}</option>}
                  {sortable.filter((field) => field.key === item.field || !draft.sort.some((sort) => sort.field === field.key)).map((field) => <option key={field.key} value={field.key}>{field.label}</option>)}
                </NativeSelect></label>
                <div className="flex items-end gap-2"><label className="grid flex-1 gap-1 text-sm"><span>{t('mobileControls.direction')}</span><NativeSelect value={item.direction ?? 'asc'} onValueChange={(direction) => changeSort(index, { direction: direction as 'asc' | 'desc' })}><option value="asc">{t('table.sortAscending')}</option><option value="desc">{t('table.sortDescending')}</option></NativeSelect></label>
                  <Button variant="ghost" aria-label={t('mobileControls.removeSort', { number: index + 1 })} onClick={() => setDraft((current) => ({ ...current, sort: current.sort.filter((_, position) => position !== index) }))}><X aria-hidden /></Button></div>
              </div>)}
              {draft.sort.length < MAX_SORTS && sortable.some((field) => !draft.sort.some((sort) => sort.field === field.key)) && <Button variant="outline" onClick={() => {
                const field = sortable.find((field) => !draft.sort.some((sort) => sort.field === field.key))
                if (field) setDraft((current) => ({ ...current, sort: [...current.sort, { field: field.key, direction: 'asc' }] }))
              }}>{t('mobileControls.addSort')}</Button>}
            </div>
          </details>
          <details className="min-w-0 border-t pt-4">
            <summary className="min-h-11 cursor-pointer text-sm font-semibold">{t('display.title')}</summary>
            <div className="grid gap-4 pt-3">
              {dateFields.length > 0 && <label className="grid gap-2 text-sm"><span>{calendar('layout.label')}</span><NativeSelect value={draft.layout} onValueChange={(layout) => setDraft((current) => ({ ...current, layout: layout as ListLayout }))}><option value="table">{calendar('layout.table')}</option><option value="calendar">{calendar('layout.calendar')}</option></NativeSelect></label>}
              {draft.layout === 'calendar' && dateFields.length > 0 && <label className="grid gap-2 text-sm"><span>{calendar('layout.field')}</span><NativeSelect value={draft.calendarField} onValueChange={(calendarField) => setDraft((current) => ({ ...current, calendarField }))}>{dateFields.map((field) => <option key={field.key} value={field.key}>{field.label}</option>)}</NativeSelect></label>}
              {draft.layout !== 'calendar' && <PageSizeSelect value={draft.perPage} onChange={(perPage) => setDraft((current) => ({ ...current, perPage }))} />}
              {draft.layout !== 'calendar' && <ColumnsMenu label={t('mobileControls.cardFields')} schema={schema} columns={draft.columns} titleField={titleField} onChange={(columns) => setDraft((current) => ({ ...current, columns }))} onReset={() => setDraft((current) => ({ ...current, columns: resetColumns }))} />}
            </div>
          </details>
          <div className="sticky bottom-0 z-10 grid grid-cols-2 gap-2 border-t bg-background pt-3 pb-1">
            <Button variant="ghost" className="col-span-2" onClick={() => { setConditions(toDraft(null)); setDraft((current) => ({ ...current, sort: [], archived: 'exclude' })) }}>{t('mobileControls.reset')}</Button>
            <Button variant="outline" onClick={() => setOpen(false)}>{common('actions.cancel')}</Button><Button type="submit">{t('filters.apply')}</Button>
          </div>
        </form>
      </DialogContent>
  </Dialog>
}
