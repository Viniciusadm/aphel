import { FieldValueDisplay } from '@/features/records/display/FieldValueDisplay'
import { fieldConfig } from '@/features/records/fields'
import { useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { CheckboxFieldInput } from './CheckboxFieldInput'
import { CurrencyFieldInput } from './CurrencyFieldInput'
import { DateFieldInput } from './DateFieldInput'
import { DateTimeFieldInput } from './DateTimeFieldInput'
import type { FieldInputProps } from './FieldInput'
import { LongTextFieldInput } from './LongTextFieldInput'
import { MultiSelectFieldInput } from './MultiSelectFieldInput'
import { NumberFieldInput } from './NumberFieldInput'
import { PhoneFieldInput } from './LazyPhoneFieldInput'
import { RecordFieldInput } from './RecordFieldInput'
import { SelectFieldInput } from './SelectFieldInput'
import { TextFieldInput } from './TextFieldInput'
import { UserFieldInput } from './UserFieldInput'
import { normalizeList, normalizeText, selectableOptions } from './lib/options'
import { toNumber } from './lib/number'

function numericValue(value: unknown): number | string | null {
  if (value === null || value === undefined || value === '') return null
  if (typeof value === 'number' || typeof value === 'string') return value
  return String(value)
}

function split({ field, value, initialValue, ...base }: FieldInputProps) {
  const config = fieldConfig(field)
  return { field, value, initialValue, base, config, maxLength: config.max_length ?? undefined, clearable: !field.is_required }
}

export function NativeTextInput(props: FieldInputProps) {
  const { value, base, maxLength } = split(props)
  return <TextFieldInput {...base} value={normalizeText(value)} maxLength={maxLength} />
}

export function NativeUrlInput(props: FieldInputProps) {
  const { value, base, maxLength } = split(props)
  return <TextFieldInput {...base} kind="url" value={normalizeText(value)} maxLength={maxLength} />
}

export function NativeEmailInput(props: FieldInputProps) {
  const { value, base, maxLength } = split(props)
  return <TextFieldInput {...base} kind="email" value={normalizeText(value)} maxLength={maxLength} />
}

export function NativeLongTextInput(props: FieldInputProps) {
  const { value, base, maxLength } = split(props)
  return <LongTextFieldInput {...base} value={normalizeText(value)} maxLength={maxLength} />
}

export function NativeNumberInput(props: FieldInputProps) {
  const formatting = useRecordFormatting()
  const { value, base, config } = split(props)
  return <NumberFieldInput {...base} value={numericValue(value)} locale={formatting.locale} decimals={config.decimals} />
}

export function NativeCurrencyInput(props: FieldInputProps) {
  const formatting = useRecordFormatting()
  const { value, base, config } = split(props)
  return (
    <CurrencyFieldInput
      {...base}
      value={toNumber(value)}
      locale={formatting.locale}
      currency={config.currency || formatting.currency}
      decimals={config.decimals}
    />
  )
}

export function NativeDateInput(props: FieldInputProps) {
  const { value, base } = split(props)
  return <DateFieldInput {...base} value={normalizeText(value)} />
}

export function NativeDateTimeInput(props: FieldInputProps) {
  const formatting = useRecordFormatting()
  const { value, base } = split(props)
  return (
    <DateTimeFieldInput {...base} value={normalizeText(value)} timeZone={formatting.timezone} locale={formatting.locale} />
  )
}

export function NativeSelectInput(props: FieldInputProps) {
  const { field, value, initialValue, base, clearable } = split(props)
  return (
    <SelectFieldInput
      {...base}
      value={normalizeText(value)}
      options={selectableOptions(field, [...normalizeList(value), ...normalizeList(initialValue)])}
      clearable={clearable}
    />
  )
}

export function NativeMultiSelectInput(props: FieldInputProps) {
  const { field, value, initialValue, base } = split(props)
  return (
    <MultiSelectFieldInput
      {...base}
      value={normalizeList(value)}
      options={selectableOptions(field, [...normalizeList(value), ...normalizeList(initialValue)])}
    />
  )
}

export function NativeCheckboxInput(props: FieldInputProps) {
  const { value, base } = split(props)
  return <CheckboxFieldInput {...base} value={value === true} />
}

export function NativePhoneInput(props: FieldInputProps) {
  const formatting = useRecordFormatting()
  const { field, value, base, config } = split(props)
  return (
    <PhoneFieldInput
      {...base}
      fieldLabel={field.label}
      value={normalizeText(value)}
      defaultCountry={config.default_country}
      locale={formatting.locale}
    />
  )
}

export function NativeUserInput(props: FieldInputProps) {
  const { value, base, clearable } = split(props)
  return <UserFieldInput {...base} value={normalizeText(value)} clearable={clearable} />
}

export function NativeRecordInput(props: FieldInputProps) {
  const { value, base, config, clearable } = split(props)
  return (
    <RecordFieldInput
      {...base}
      value={normalizeText(value)}
      targetRecordType={config.target_record_type}
      clearable={clearable}
    />
  )
}

export function NativeFormulaInput({ field, value, ...rest }: FieldInputProps) {
  return (
    <div
      id={rest.id}
      role="textbox"
      aria-readonly="true"
      aria-label={rest['aria-label']}
      aria-labelledby={rest['aria-labelledby']}
      aria-describedby={rest['aria-describedby']}
      className={cn(
        'flex min-h-9 min-w-0 items-center rounded-md border border-input bg-muted/40 px-3 py-1.5 text-sm',
        rest.className,
      )}
    >
      <FieldValueDisplay field={field} value={value} />
    </div>
  )
}
