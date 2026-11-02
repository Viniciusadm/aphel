import { X } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { SEARCH_MIN_LENGTH, useGlobalSearch } from '@/features/search/api'
import { useSearchRecordTypeLabel } from '@/features/search/links'
import { checkInRecordTypes } from '../lib/record-route'
import { Field } from '@/components/ui/field'
import { SearchInput } from '@/components/ui/search-input'

export interface PickedRecord {
  recordType: string
  recordId: string
  title: string
}

export interface RecordPickerProps {
  value: PickedRecord | null
  onChange: (value: PickedRecord | null) => void
  required?: boolean
}

const PICKER_LIMIT = 5

export function RecordPicker({ value, onChange, required = false }: RecordPickerProps) {
  const { t } = useTranslation('mobile')
  const typeLabel = useSearchRecordTypeLabel()
  const inputId = useId()
  const [query, setQuery] = useState('')
  const search = useGlobalSearch({ q: value ? '' : query, types: checkInRecordTypes(), limit: PICKER_LIMIT })
  const hits = (search.data?.groups ?? []).flatMap((group) => group.items)
  const searching = !value && query.trim().length >= SEARCH_MIN_LENGTH

  return (
    <Field id={inputId} label={t(required ? 'picker.labelRequired' : 'picker.label')}>
      {value ? (
        <div className="flex min-h-11 items-center gap-2 rounded-md border bg-muted/40 px-3">
          <span className="min-w-0 flex-1 truncate text-sm">
            <span className="text-muted-foreground">{typeLabel(value.recordType)} · </span>
            {value.title}
          </span>
          <Button type="button" variant="ghost" size="icon" onClick={() => onChange(null)} aria-label={t('picker.clear')}>
            <X aria-hidden="true" />
          </Button>
        </div>
      ) : (
        <SearchInput
            id={inputId}
            value={query}
            onValueChange={(value) => setQuery(value)}
            placeholder={t('picker.placeholder')}
            className="h-11  text-base"
            autoComplete="off"
          />
      )}
      {searching && (
        <ul className="grid divide-y rounded-md border" aria-label={t('picker.results')} aria-busy={search.isFetching}>
          {hits.length === 0 ? (
            <li className="px-3 py-3 text-sm text-muted-foreground">
              {search.isFetching ? t('picker.searching') : t('picker.empty')}
            </li>
          ) : (
            hits.map((hit) => (
              <li key={`${hit.record_type}:${hit.id}`}>
                <button
                  type="button"
                  className="grid min-h-11 w-full gap-0.5 px-3 py-2 text-left hover:bg-accent focus-visible:bg-accent focus-visible:outline-none"
                  onClick={() => {
                    onChange({ recordType: hit.record_type, recordId: hit.id, title: hit.title })
                    setQuery('')
                  }}
                >
                  <span className="truncate text-sm font-medium">{hit.title}</span>
                  <span className="truncate text-xs text-muted-foreground">
                    {typeLabel(hit.record_type)}
                    {hit.subtitle ? ` · ${hit.subtitle}` : ''}
                  </span>
                </button>
              </li>
            ))
          )}
        </ul>
      )}
    </Field>
  )
}
