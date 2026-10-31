import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { DURATION_UNITS, formatDuration, parseDuration, type DurationSign, type DurationUnit } from '../lib/durations'

export interface DurationInputProps {
  id?: string
  value: string | null | undefined
  onChange: (value: string | null) => void
  signed?: boolean
  invalid?: boolean
  'aria-describedby'?: string
}

export function DurationInput({ id, value, onChange, signed = false, invalid, ...rest }: DurationInputProps) {
  const { t } = useTranslation('automations')
  const parsed = parseDuration(value) ?? { sign: '+' as DurationSign, amount: Number.NaN, unit: 'd' as DurationUnit }
  const amountText = Number.isFinite(parsed.amount) ? String(parsed.amount) : ''

  const emit = (next: { sign?: DurationSign; amount?: string; unit?: DurationUnit }) => {
    const amountValue = next.amount ?? amountText
    if (amountValue.trim() === '') {
      onChange(null)
      return
    }
    const amount = Math.max(0, Math.floor(Number(amountValue)))
    if (!Number.isFinite(amount)) return
    onChange(
      formatDuration({ sign: next.sign ?? parsed.sign, amount, unit: next.unit ?? parsed.unit }, signed),
    )
  }

  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <Input placeholder={t('placeholders.amount')}
        id={id}
        type="number"
        inputMode="numeric"
        min={0}
        value={amountText}
        onChange={(event) => emit({ amount: event.target.value })}
        aria-invalid={invalid ? true : undefined}
        aria-describedby={rest['aria-describedby']}
        className="w-24"
      />
      <NativeSelect
        value={parsed.unit}
        onChange={(event) => emit({ unit: event.target.value as DurationUnit })}
        aria-label={t('duration.unit')}
        className="w-32"
      >
        {DURATION_UNITS.map((unit) => (
          <option key={unit} value={unit}>
            {t(`duration.units.${unit}`)}
          </option>
        ))}
      </NativeSelect>
      {signed && (
        <NativeSelect
          value={parsed.sign}
          onChange={(event) => emit({ sign: event.target.value as DurationSign })}
          aria-label={t('duration.direction')}
          className="w-32"
        >
          <option value="-">{t('duration.before')}</option>
          <option value="+">{t('duration.after')}</option>
        </NativeSelect>
      )}
    </div>
  )
}
