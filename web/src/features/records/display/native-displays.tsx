import { Check, Minus } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { FieldDisplayProps } from '@/kernel/modules/field-types'
import { fieldConfig } from '../fields'
import { useRecordFormatting } from '../formatting'
import { OptionChip, RecordValue, UserValue } from './FieldValueDisplay'
import { formatFieldText, optionFor } from './values'

export function CheckboxValue({ value }: FieldDisplayProps) {
  const { t } = useTranslation('records')
  const checked = value === true
  return (
    <span className="inline-flex items-center gap-1">
      {checked ? <Check className="size-4" aria-hidden="true" /> : <Minus className="size-4 text-muted-foreground" aria-hidden="true" />}
      <span className="sr-only">{checked ? t('values.yes') : t('values.no')}</span>
    </span>
  )
}

export function SelectValue({ field, value }: FieldDisplayProps) {
  const option = optionFor(field, value)
  return option ? <OptionChip option={option} /> : String(value)
}

export function MultiSelectValue({ field, value }: FieldDisplayProps) {
  const values = Array.isArray(value) ? value : [value]
  return (
    <span className="inline-flex flex-wrap gap-1">
      {values.map((item) => {
        const option = optionFor(field, item)
        return option ? <OptionChip key={String(item)} option={option} /> : <span key={String(item)}>{String(item)}</span>
      })}
    </span>
  )
}

export function UserFieldValue({ value }: FieldDisplayProps) {
  return <UserValue userId={String(value)} />
}

export function RecordFieldValue({ field, value }: FieldDisplayProps) {
  const target = fieldConfig(field).target_record_type
  return target ? <RecordValue recordType={target} recordId={String(value)} /> : String(value)
}

export function UrlValue({ value }: FieldDisplayProps) {
  return (
    <a href={String(value)} target="_blank" rel="noreferrer noopener" className="truncate text-primary underline-offset-4 hover:underline">
      {String(value).replace(/^https?:\/\//, '')}
    </a>
  )
}

export function EmailValue({ value }: FieldDisplayProps) {
  return (
    <a href={`mailto:${String(value)}`} className="truncate text-primary underline-offset-4 hover:underline">
      {String(value)}
    </a>
  )
}

type PhoneFormatter = (value: string) => string

let phoneFormatter: PhoneFormatter | null = null
let phoneFormatterLoad: Promise<PhoneFormatter> | null = null

function loadPhoneFormatter(): Promise<PhoneFormatter> {
  phoneFormatterLoad ??= import('libphonenumber-js').then(({ parsePhoneNumberFromString }) => {
    phoneFormatter = (value) => parsePhoneNumberFromString(value)?.formatInternational() ?? value
    return phoneFormatter
  })
  return phoneFormatterLoad
}

function useFormattedPhone(value: string): string {
  const [formatter, setFormatter] = useState<PhoneFormatter | null>(() => phoneFormatter)
  useEffect(() => {
    if (formatter) return
    let active = true
    void loadPhoneFormatter().then((loaded) => {
      if (active) setFormatter(() => loaded)
    })
    return () => {
      active = false
    }
  }, [formatter])
  return formatter ? formatter(value) : value
}

export function PhoneValue({ value }: FieldDisplayProps) {
  const raw = String(value)
  const formatted = useFormattedPhone(raw)
  return (
    <a href={`tel:${raw}`} className="truncate underline-offset-4 hover:underline">
      {formatted}
    </a>
  )
}

export function LongTextValue({ value }: FieldDisplayProps) {
  return <span className="line-clamp-3 whitespace-pre-line">{String(value)}</span>
}

export function NumericValue({ field, value }: FieldDisplayProps) {
  const formatting = useRecordFormatting()
  return <span className="tabular-nums">{formatFieldText(field, value, formatting)}</span>
}
