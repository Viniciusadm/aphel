import { useTranslation } from 'react-i18next'
import { toast } from '@/components/ui/toast-store'
import { useUpdateRecord } from '@/features/records/detail'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatCurrency, formatNumber, useRecordFormatting } from '@/features/records/formatting'
import type { RecordItem } from '@/features/records/types'
import { useCurrencySettings } from './api'
import { currencyErrorKey } from './errors'

const DEAL_RECORD_TYPE = 'deals.deal'
const RATE_DECIMALS = 10

interface DealCurrencyControlProps {
  record: RecordItem
  readOnly?: boolean
}

function text(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null
}

export function DealCurrencyControl({ record, readOnly = false }: DealCurrencyControlProps) {
  const { t } = useTranslation('currencies')
  const formatting = useRecordFormatting()
  const settings = useCurrencySettings()
  const update = useUpdateRecord(DEAL_RECORD_TYPE, record.id)
  const currency = text(record.currency) ?? ''
  const amountBase = text(record.amount_base)
  const rate = text(record.exchange_rate)
  const base = settings.data?.base
  const options = settings.data?.currencies.map((item) => item.code) ?? []
  const choices = currency && !options.includes(currency) ? [currency, ...options] : options
  const converted = base !== undefined && currency !== base && text(record.amount) !== null

  const change = async (next: string) => {
    if (next === currency) return
    try {
      await update.mutateAsync({ currency: next })
    } catch (error) {
      toast({ title: t(currencyErrorKey(error)), variant: 'destructive' })
    }
  }

  return (
    <div className="flex flex-wrap items-center gap-2 text-sm">
      <NativeSelect
        aria-label={t('deal.currency')}
        value={currency}
        disabled={readOnly || update.isPending || choices.length <= 1}
        onChange={(event) => void change(event.target.value)}
      >
        {choices.map((code) => (
          <option key={code} value={code}>
            {code}
          </option>
        ))}
      </NativeSelect>
      {converted && (
        <span className="text-muted-foreground tabular-nums">
          {amountBase !== null && rate !== null
            ? t('deal.converted', {
                amount: formatCurrency(amountBase, formatting, base),
                rate: formatNumber(rate, formatting, RATE_DECIMALS),
              })
            : t('deal.notConverted', { base })}
        </span>
      )}
    </div>
  )
}
