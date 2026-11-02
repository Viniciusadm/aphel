import { useQuery } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { recordsKeys } from '@/features/records/api'

import { getRecordResource, recordPath, recordTitle } from '@/features/records/resources'
import type { Page, RecordItem } from '@/features/records/types'
import { SearchablePicker } from './SearchablePicker'
import type { FieldInputBaseProps } from './types'

export const RECORD_SEARCH_PAGE_SIZE = 20
export const RECORD_SEARCH_DEBOUNCE_MS = 250

export interface RecordFieldInputProps extends FieldInputBaseProps<string> {
  targetRecordType: string | null | undefined
  clearable?: boolean
}

function useDebounced<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay)
    return () => clearTimeout(timer)
  }, [value, delay])
  return debounced
}

export function RecordFieldInput({ targetRecordType, clearable = true, ...props }: RecordFieldInputProps) {
  const { t } = useTranslation('records-form')
  const resource = targetRecordType ? getRecordResource(targetRecordType) : undefined
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const [picked, setPicked] = useState<Record<string, string>>({})
  const term = useDebounced(search.trim(), RECORD_SEARCH_DEBOUNCE_MS)
  const recordType = resource?.recordType ?? ''

  const results = useQuery({
    queryKey: [...recordsKeys.lookup(recordType), term],
    queryFn: ({ signal }) =>
      api.get<Page<RecordItem>>(resource!.basePath, {
        query: { search: term || undefined, per_page: RECORD_SEARCH_PAGE_SIZE },
        signal,
      }),
    enabled: open && Boolean(resource),
    staleTime: 30_000,
  })

  const selectedId = props.value
  const current = useQuery({
    queryKey: [...recordsKeys.detail(recordType, selectedId ?? '')],
    queryFn: () => api.get<RecordItem>(recordPath(resource!, selectedId!)),
    enabled: Boolean(resource && selectedId && !picked[selectedId]),
    staleTime: 60_000,
  })

  if (!resource) {
    return (
      <Input
        id={props.id}
        disabled
        value={selectedId ?? ''}
        placeholder={t('input.recordUnavailable')}
        aria-describedby={props['aria-describedby']}
        readOnly
      />
    )
  }

  const items = results.data?.data ?? []
  const selectedLabel = selectedId
    ? (picked[selectedId] ?? (current.data ? recordTitle(resource, current.data) : selectedId))
    : null

  return (
    <div className="flex min-w-0 items-center gap-1">
      <SearchablePicker
        id={props.id}
        open={open}
        onOpenChange={setOpen}
        search={search}
        onSearchChange={setSearch}
        selected={selectedId ? [selectedId] : []}
        onSelect={(value) => {
          const record = items.find((item) => item.id === value)
          if (record) setPicked((previous) => ({ ...previous, [value]: recordTitle(resource, record) }))
          props.onChange(value)
        }}
        options={items.map((item) => ({ value: item.id, label: recordTitle(resource, item) }))}
        loading={results.isPending && results.fetchStatus !== 'idle'}
        errorMessage={results.isError ? t('input.loadError') : undefined}
        disabled={props.disabled}
        invalid={props.invalid}
        autoFocus={props.autoFocus}
        className={props.className}
        onBlur={props.onBlur}
        aria-describedby={props['aria-describedby']}
        aria-labelledby={props['aria-labelledby']}
        aria-label={props['aria-label']}
      >
        {selectedLabel ? (
          <span className="truncate">{selectedLabel}</span>
        ) : (
          <span className="text-muted-foreground">{t('input.placeholder')}</span>
        )}
      </SearchablePicker>
      {clearable && selectedId && !props.disabled && (
        <Button variant="ghost" size="icon" aria-label={t('input.clear')} onClick={() => props.onChange(null)}>
          <X />
        </Button>
      )}
    </div>
  )
}
