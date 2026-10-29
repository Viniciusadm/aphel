import { Search, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

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
    <div className={cn('relative min-w-0', className)}>
      <Search
        className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
        aria-hidden
      />
      <Input
        type="search"
        aria-label={t('search.label')}
        placeholder={t('search.placeholder')}
        maxLength={MAX_SEARCH_LENGTH}
        className="h-8 pr-8 pl-8 [&::-webkit-search-cancel-button]:hidden"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') onChangeRef.current(draft)
        }}
      />
      {draft && (
        <button
          type="button"
          className="absolute top-1/2 right-2 flex size-5 -translate-y-1/2 items-center justify-center rounded-sm text-muted-foreground hover:text-foreground"
          aria-label={t('filters.clear')}
          onClick={() => {
            setDraft('')
            onChangeRef.current('')
          }}
        >
          <X className="size-3.5" aria-hidden />
        </button>
      )}
    </div>
  )
}
