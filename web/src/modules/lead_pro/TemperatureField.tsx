import type { FieldInputProps } from '@/components/fields/FieldInput'
import { Input } from '@/components/ui/input'
import type { FieldDisplayProps } from '@/kernel/modules/field-types'
import { MAX_TEMPERATURE } from './api'
import { DEFAULT_HOT_THRESHOLD, TemperatureBadge } from './TemperatureBadge'

function asTemperature(value: unknown): number | null {
  const number = typeof value === 'number' ? value : typeof value === 'string' ? Number(value) : Number.NaN
  return Number.isFinite(number) ? number : null
}

export function TemperatureDisplay({ value }: FieldDisplayProps) {
  const temperature = asTemperature(value)
  if (temperature === null) return null
  return <TemperatureBadge temperature={temperature} hotThreshold={DEFAULT_HOT_THRESHOLD} />
}

export function TemperatureInput({ value, onChange, field: _field, initialValue: _initialValue, ...props }: FieldInputProps) {
  const temperature = asTemperature(value)
  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        min={0}
        max={MAX_TEMPERATURE}
        step={1}
        value={temperature ?? 0}
        onChange={(event) => onChange(Number(event.target.value))}
        className="h-2 flex-1 cursor-pointer accent-primary"
        aria-label={props['aria-label']}
        aria-labelledby={props['aria-labelledby']}
        disabled={props.disabled}
      />
      <Input
        placeholder="0"
        {...props}
        type="number"
        min={0}
        max={MAX_TEMPERATURE}
        className="w-20 tabular-nums"
        value={temperature ?? ''}
        onChange={(event) => onChange(event.target.value === '' ? null : Number(event.target.value))}
      />
    </div>
  )
}
