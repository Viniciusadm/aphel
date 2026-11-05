import { Repeat } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { formatCurrency, NativeSelect, useRecordFormatting } from '@/sdk'
import { MAX_TERM_MONTHS, RECURRENCES, type Recurrence, type RecurringItemFields, type RecurringItemRequest } from './types'

export interface ItemRecurrenceFieldsProps {
  item: RecurringItemFields
  currency: string
  disabled: boolean
  onSave: (body: RecurringItemRequest) => Promise<void>
}

function parseTerm(text: string): number | null | undefined {
  const value = text.trim()
  if (value === '') return null
  const months = Number(value)
  if (!Number.isInteger(months) || months < 1 || months > MAX_TERM_MONTHS) return undefined
  return months
}

export function ItemRecurrenceFields({ item, currency, disabled, onSave }: ItemRecurrenceFieldsProps) {
  const { t } = useTranslation('line_items')
  const formatting = useRecordFormatting()
  const recurrence = item.recurrence ?? 'none'
  const [term, setTerm] = useState(item.term_months ? String(item.term_months) : '')
  const recurring = recurrence !== 'none'

  const save = async (body: RecurringItemRequest) => {
    try {
      await onSave(body)
    } catch {
      setTerm(item.term_months ? String(item.term_months) : '')
    }
  }

  const commitTerm = () => {
    const months = parseTerm(term)
    if (months === undefined) {
      setTerm(item.term_months ? String(item.term_months) : '')
      return
    }
    if (months !== (item.term_months ?? null)) void save({ term_months: months })
  }

  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <Repeat className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
      <NativeSelect
        aria-label={t('item.recurrence')}
        className="w-36 shrink-0"
        value={recurrence}
        disabled={disabled}
        onChange={(event) => void save({ recurrence: event.target.value as Recurrence })}
      >
        {RECURRENCES.map((value) => (
          <option key={value} value={value}>
            {t(`recurrences.${value}`)}
          </option>
        ))}
      </NativeSelect>
      {recurring && (
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span>{t('item.term')}</span>
          <Input
            aria-label={t('item.termLabel')}
            inputMode="numeric"
            className="h-9 w-20 tabular-nums"
            placeholder={t('item.termPlaceholder')}
            value={term}
            disabled={disabled}
            onChange={(event) => setTerm(event.target.value)}
            onBlur={commitTerm}
            onKeyDown={(event) => {
              if (event.key === 'Enter') event.currentTarget.blur()
            }}
          />
          <span>{t('item.months')}</span>
        </label>
      )}
      {recurring && item.monthly_amount && (
        <span className="text-xs text-muted-foreground tabular-nums">
          {t('item.monthly', { amount: formatCurrency(item.monthly_amount, formatting, currency) })}
        </span>
      )}
    </div>
  )
}
