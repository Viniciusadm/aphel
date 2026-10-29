import { Check, ChevronDown, Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import type { FieldDefinition } from '../types'

export interface FieldPickerProps {
  fields: FieldDefinition[]
  value: string
  onChange: (field: FieldDefinition) => void
  className?: string
}

function normalize(text: string) {
  return text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
}

export function FieldPicker({ fields, value, onChange, className }: FieldPickerProps) {
  const { t } = useTranslation('records-table')
  const [open, setOpen] = useState(false)
  const [term, setTerm] = useState('')
  const selected = fields.find((field) => field.key === value)
  const visible = useMemo(() => {
    const needle = normalize(term.trim())
    if (!needle) return fields
    return fields.filter((field) => normalize(field.label).includes(needle) || field.key.includes(needle))
  }, [fields, term])

  const choose = (field: FieldDefinition) => {
    onChange(field)
    setOpen(false)
    setTerm('')
  }

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setTerm('')
      }}
    >
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          aria-label={selected ? `${t('filters.field')}: ${selected.label}` : t('filters.field')}
          className={cn('w-full justify-between font-normal sm:w-44', !selected && 'text-muted-foreground', className)}
        >
          <span className="truncate">{selected?.label ?? t('filters.fieldPlaceholder')}</span>
          <ChevronDown className="text-muted-foreground" aria-hidden />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-64 p-2">
        <div className="relative mb-2">
          <Search
            className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden
          />
          <Input
            autoFocus
            aria-label={t('filters.fieldSearch')}
            placeholder={t('filters.fieldSearch')}
            className="pl-8"
            value={term}
            onChange={(event) => setTerm(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && visible[0]) {
                event.preventDefault()
                choose(visible[0])
              }
            }}
          />
        </div>
        <ul aria-label={t('filters.field')} className="grid max-h-60 gap-0.5 overflow-y-auto">
          {visible.length === 0 && <li className="px-2 py-1.5 text-sm text-muted-foreground">{t('filters.noFields')}</li>}
          {visible.map((field) => (
            <li key={field.key}>
              <button
                type="button"
                aria-current={field.key === value ? 'true' : undefined}
                className="flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-left text-sm outline-none hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent"
                onClick={() => choose(field)}
              >
                <span className="truncate">{field.label}</span>
                {field.key === value && <Check className="size-4 shrink-0" aria-hidden />}
              </button>
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  )
}
