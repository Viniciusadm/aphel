import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { recordTypeLabel, KNOWN_RECORD_TYPES } from './lib/labels'
import { EMPTY_FILTERS, hasFilters, type AuditFilters } from './lib/filters'

interface AuditFiltersFormProps {
  filters: AuditFilters
  onApply: (filters: AuditFilters) => void
}

export function AuditFiltersForm({ filters, onApply }: AuditFiltersFormProps) {
  const { t } = useTranslation('audit')
  const id = useId()
  const [draft, setDraft] = useState(filters)

  const update = (field: keyof AuditFilters) => (event: { target: { value: string } }) =>
    setDraft((current) => ({ ...current, [field]: event.target.value }))

  const submit = (event: FormEvent) => {
    event.preventDefault()
    onApply(draft)
  }

  const clear = () => {
    setDraft(EMPTY_FILTERS)
    onApply(EMPTY_FILTERS)
  }

  return (
    <form
      onSubmit={submit}
      aria-label={t('filters.label')}
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-2 lg:grid-cols-5"
    >
      <div className="grid gap-1.5">
        <Label htmlFor={`${id}-record-type`}>{t('filters.recordType')}</Label>
        <Input
          id={`${id}-record-type`}
          list={`${id}-record-types`}
          value={draft.recordType}
          onChange={update('recordType')}
          placeholder={t('filters.recordTypePlaceholder')}
          autoComplete="off"
        />
        <datalist id={`${id}-record-types`}>
          {KNOWN_RECORD_TYPES.map((recordType) => {
            const labelKey = recordTypeLabel(recordType)
            return (
              <option key={recordType} value={recordType}>
                {labelKey ? t(labelKey) : recordType}
              </option>
            )
          })}
        </datalist>
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor={`${id}-record-id`}>{t('filters.recordId')}</Label>
        <Input
          id={`${id}-record-id`}
          value={draft.recordId}
          onChange={update('recordId')}
          placeholder={t('filters.recordIdPlaceholder')}
          autoComplete="off"
        />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor={`${id}-actor`}>{t('filters.actor')}</Label>
        <Input
          id={`${id}-actor`}
          value={draft.actor}
          onChange={update('actor')}
          placeholder={t('filters.actorPlaceholder')}
          autoComplete="off"
        />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor={`${id}-from`}>{t('filters.from')}</Label>
        <Input id={`${id}-from`} type="date" value={draft.from} onChange={update('from')} max={draft.to || undefined} />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor={`${id}-to`}>{t('filters.to')}</Label>
        <Input id={`${id}-to`} type="date" value={draft.to} onChange={update('to')} min={draft.from || undefined} />
      </div>
      <div className="flex flex-wrap justify-end gap-2 sm:col-span-2 lg:col-span-5">
        <Button type="button" variant="outline" onClick={clear} disabled={!hasFilters(draft) && !hasFilters(filters)}>
          {t('filters.clear')}
        </Button>
        <Button type="submit">{t('filters.apply')}</Button>
      </div>
    </form>
  )
}
