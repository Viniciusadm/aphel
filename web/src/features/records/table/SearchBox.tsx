import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { SearchInput } from '@/components/ui/search-input'

export const SEARCH_DEBOUNCE_MS = 300
export const MAX_SEARCH_LENGTH = 100

export interface SearchBoxProps {
  value: string
  onChange: (value: string) => void
  delay?: number
  className?: string
}

export function SearchBox({ value, onChange, delay = SEARCH_DEBOUNCE_MS, className }: SearchBoxProps) {
  const { t } = useTranslation('records-table')
  const [draft, setDraft] = useState(value)
  const [synced, setSynced] = useState(value)
  const onChangeRef = useRef(onChange)

  if (value !== synced) {
    setSynced(value)
    setDraft(value)
  }

  useEffect(() => {
    onChangeRef.current = onChange
  })

  useEffect(() => {
    if (draft === value) return
    const timer = window.setTimeout(() => onChangeRef.current(draft), delay)
    return () => window.clearTimeout(timer)
  }, [draft, value, delay])

  return (
    <SearchInput
      wrapperClassName={className}
      aria-label={t('search.label')}
      placeholder={t('search.placeholder')}
      maxLength={MAX_SEARCH_LENGTH}
      clearLabel={t('filters.clear')}
      value={draft}
      onValueChange={(next) => {
        setDraft(next)
        if (next === '') onChangeRef.current('')
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') onChangeRef.current(draft)
      }}
    />
  )
}
