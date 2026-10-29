import { Check, ChevronsUpDown } from 'lucide-react'
import { useId, useState, type KeyboardEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { cn } from '@/lib/utils'

export interface PickerOption {
  value: string
  label: string
  content?: ReactNode
}

export interface SearchablePickerProps {
  id?: string
  options: PickerOption[]
  selected: string[]
  onSelect: (value: string) => void
  search: string
  onSearchChange: (search: string) => void
  open: boolean
  onOpenChange: (open: boolean) => void
  children: ReactNode
  multiple?: boolean
  loading?: boolean
  errorMessage?: string
  disabled?: boolean
  invalid?: boolean
  autoFocus?: boolean
  className?: string
  onBlur?: () => void
  'aria-describedby'?: string
  'aria-labelledby'?: string
  'aria-label'?: string
}

export function SearchablePicker({
  id,
  options,
  selected,
  onSelect,
  search,
  onSearchChange,
  open,
  onOpenChange,
  children,
  multiple = false,
  loading = false,
  errorMessage,
  disabled,
  invalid,
  autoFocus,
  className,
  onBlur,
  ...aria
}: SearchablePickerProps) {
  const { t } = useTranslation('records-form')
  const listId = useId()
  const [active, setActive] = useState(0)
  const activeIndex = Math.min(active, options.length - 1)
  const activeOption = activeIndex >= 0 ? options[activeIndex] : undefined
  const optionId = (index: number) => `${listId}-option-${index}`

  const choose = (option: PickerOption) => {
    onSelect(option.value)
    if (!multiple) {
      onOpenChange(false)
      onSearchChange('')
    }
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setActive(Math.min(activeIndex + 1, options.length - 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setActive(Math.max(activeIndex - 1, 0))
    } else if (event.key === 'Enter') {
      event.preventDefault()
      event.stopPropagation()
      if (activeOption) choose(activeOption)
    } else if (event.key === 'Escape') {
      event.stopPropagation()
    }
  }

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        onOpenChange(next)
        if (!next) onBlur?.()
      }}
    >
      <PopoverTrigger asChild>
        <button
          id={id}
          type="button"
          disabled={disabled}
          autoFocus={autoFocus}
          aria-haspopup="listbox"
          aria-expanded={open}
          aria-invalid={invalid ? true : undefined}
          aria-describedby={aria['aria-describedby']}
          aria-labelledby={aria['aria-labelledby']}
          aria-label={aria['aria-label']}
          className={cn(
            'flex min-h-9 w-full min-w-0 items-center justify-between gap-2 rounded-md border border-input bg-background px-3 py-1.5 text-left text-sm shadow-xs outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive',
            className,
          )}
        >
          <span className="flex min-w-0 flex-1 flex-wrap items-center gap-1">{children}</span>
          <ChevronsUpDown className="size-4 shrink-0 opacity-50" aria-hidden="true" />
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-(--radix-popover-trigger-width) min-w-56 p-0">
        <div className="border-b p-2">
          <Input
            role="combobox"
            aria-label={t('input.searchLabel')}
            aria-autocomplete="list"
            aria-expanded={open}
            aria-controls={listId}
            aria-activedescendant={activeOption ? optionId(activeIndex) : undefined}
            placeholder={t('input.search')}
            value={search}
            onChange={(event) => {
              onSearchChange(event.target.value)
              setActive(0)
            }}
            onKeyDown={handleKeyDown}
          />
        </div>
        <div
          id={listId}
          role="listbox"
          aria-multiselectable={multiple || undefined}
          className="max-h-60 overflow-y-auto p-1"
        >
          {loading ? (
            <p className="px-2 py-3 text-sm text-muted-foreground">{t('input.loading')}</p>
          ) : errorMessage ? (
            <p className="px-2 py-3 text-sm text-destructive">{errorMessage}</p>
          ) : options.length === 0 ? (
            <p className="px-2 py-3 text-sm text-muted-foreground">{t('input.noResults')}</p>
          ) : (
            options.map((option, index) => {
              const isSelected = selected.includes(option.value)
              return (
                <div
                  key={option.value}
                  id={optionId(index)}
                  role="option"
                  aria-selected={isSelected}
                  data-active={index === activeIndex || undefined}
                  className="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none select-none hover:bg-accent hover:text-accent-foreground data-[active]:bg-accent data-[active]:text-accent-foreground"
                  onMouseMove={() => setActive(index)}
                  onClick={() => choose(option)}
                >
                  <Check className={cn('size-4 shrink-0', isSelected ? 'opacity-100' : 'opacity-0')} aria-hidden="true" />
                  <span className="min-w-0 flex-1 truncate">{option.content ?? option.label}</span>
                </div>
              )
            })
          )}
        </div>
      </PopoverContent>
    </Popover>
  )
}
