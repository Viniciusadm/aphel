import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import {
  PHONE_COUNTRIES,
  countryName,
  findPhoneCountry,
  formatNationalNumber,
  isPhoneValid,
  limitNationalDigits,
  resolvePhoneCountry,
  splitPhoneNumber,
  toE164,
  type PhoneCountry,
} from './lib/phone'
import { ariaProps, type FieldInputBaseProps } from './types'

export interface PhoneFieldInputProps extends FieldInputBaseProps<string> {
  defaultCountry?: string | null
  locale: string
  fieldLabel?: string
}

export function PhoneFieldInput({ defaultCountry, locale, fieldLabel, ...props }: PhoneFieldInputProps) {
  const { t } = useTranslation('records-form')
  const fallback = resolvePhoneCountry(defaultCountry)
  const [chosenCountry, setChosenCountry] = useState<PhoneCountry | null>(null)
  const parts = splitPhoneNumber(props.value, chosenCountry ?? fallback)
  const country = props.value ? parts.country : (chosenCountry ?? fallback)
  const display = formatNationalNumber(country, parts.national)
  const [touched, setTouched] = useState(false)
  const names = useMemo(
    () => new Map<string, string>(PHONE_COUNTRIES.map((item) => [item.code, countryName(item.code, locale)])),
    [locale],
  )
  const countries = useMemo(
    () =>
      [...PHONE_COUNTRIES].sort((left, right) =>
        (names.get(left.code) ?? left.code).localeCompare(names.get(right.code) ?? right.code, locale),
      ),
    [names, locale],
  )
  const invalid = touched && Boolean(props.value) && !isPhoneValid(props.value)

  const emit = (target: PhoneCountry, national: string) => props.onChange(toE164(target, national))

  return (
    <div className={cn('flex min-w-0 gap-2', props.className)}>
      <select
        aria-label={fieldLabel ? t('input.countryOf', { field: fieldLabel }) : t('input.country')}
        disabled={props.disabled}
        className="h-9 w-24 shrink-0 rounded-md border border-input bg-background px-2 text-sm shadow-xs outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
        value={country.code}
        onChange={(event) => {
          const next = findPhoneCountry(event.target.value) ?? fallback
          setChosenCountry(next)
          if (parts.national) emit(next, parts.national)
        }}
      >
        {countries.map((item) => (
          <option key={item.code} value={item.code}>
            {`${names.get(item.code)} (+${item.dialCode})`}
          </option>
        ))}
      </select>
      <Input
        {...ariaProps(props)}
        aria-invalid={invalid || props.invalid || undefined}
        type="tel"
        placeholder={t('input.phonePlaceholder')}
        inputMode="tel"
        autoComplete="tel-national"
        className="min-w-0 flex-1"
        value={display}
        onBlur={() => {
          setTouched(true)
          props.onBlur?.()
        }}
        onChange={(event) => {
          const text = event.target.value
          if (text.trim().startsWith('+')) {
            const pasted = splitPhoneNumber(text, country)
            setChosenCountry(pasted.country)
            emit(pasted.country, pasted.national)
            return
          }
          let digits = text.replace(/\D/g, '')
          if (digits === parts.national && text.length < display.length) digits = digits.slice(0, -1)
          setChosenCountry(country)
          emit(country, limitNationalDigits(country, digits))
        }}
      />
    </div>
  )
}
