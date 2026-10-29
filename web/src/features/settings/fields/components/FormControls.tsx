import type { ComponentProps, KeyboardEventHandler, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { cn } from '@/lib/utils'
import { HEX_COLOR_PATTERN } from '../lib/form'
import { describedBy } from '../lib/aria'
import { asFieldsMessageKey } from '../lib/errors'

interface FieldMessageProps {
  id: string
  message?: string
}

export function FieldMessage({ id, message }: FieldMessageProps) {
  const { t } = useTranslation('fields')
  const key = asFieldsMessageKey(message)
  if (!key) return null
  return (
    <p id={id} className="text-sm text-destructive">
      {t(key)}
    </p>
  )
}

export function FormAlert({ message }: { message?: string }) {
  const { t } = useTranslation('fields')
  const key = asFieldsMessageKey(message)
  if (!key) return null
  return (
    <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {t(key)}
    </p>
  )
}

interface FormFieldProps {
  id: string
  label: string
  hint?: string
  error?: string
  className?: string
  children: ReactNode
}

export function FormField({ id, label, hint, error, className, children }: FormFieldProps) {
  return (
    <div className={cn('grid content-start gap-2', className)}>
      <Label htmlFor={id}>{label}</Label>
      {children}
      {error ? (
        <FieldMessage id={`${id}-error`} message={error} />
      ) : (
        hint && (
          <p id={`${id}-hint`} className="text-xs text-muted-foreground">
            {hint}
          </p>
        )
      )}
    </div>
  )
}

interface CheckboxFieldProps {
  id: string
  label: string
  hint?: string
  error?: string
  disabled?: boolean
  inputProps: Omit<ComponentProps<'input'>, 'type' | 'id'>
}

export function CheckboxField({ id, label, hint, error, disabled, inputProps }: CheckboxFieldProps) {
  return (
    <div className="flex items-start gap-3 rounded-md border px-3 py-2">
      <input
        id={id}
        type="checkbox"
        className="mt-0.5 size-4 shrink-0 accent-primary"
        disabled={disabled}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy(id, error, hint)}
        {...inputProps}
      />
      <div className="grid gap-1">
        <label htmlFor={id} className="text-sm leading-none font-medium">
          {label}
        </label>
        {error ? (
          <FieldMessage id={`${id}-error`} message={error} />
        ) : (
          hint && (
            <p id={`${id}-hint`} className="text-xs text-muted-foreground">
              {hint}
            </p>
          )
        )}
      </div>
    </div>
  )
}

const NONE_VALUE = '__none__'

export interface Choice {
  value: string
  label: string
}

interface ChoiceSelectProps {
  id: string
  value: string
  choices: Choice[]
  noneLabel?: string
  placeholder?: string
  invalid?: boolean
  describedBy?: string
  disabled?: boolean
  onChange: (value: string) => void
}

export function ChoiceSelect({
  id,
  value,
  choices,
  noneLabel,
  placeholder,
  invalid,
  describedBy: describedById,
  disabled,
  onChange,
}: ChoiceSelectProps) {
  const selected = value === '' && noneLabel ? NONE_VALUE : value
  return (
    <Select value={selected} onValueChange={(next) => onChange(next === NONE_VALUE ? '' : next)} disabled={disabled}>
      <SelectTrigger id={id} aria-invalid={invalid || undefined} aria-describedby={describedById}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {noneLabel && <SelectItem value={NONE_VALUE}>{noneLabel}</SelectItem>}
        {choices.map((choice) => (
          <SelectItem key={choice.value} value={choice.value}>
            {choice.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

const FALLBACK_COLOR = '#94a3b8'

interface ColorInputProps {
  id?: string
  value: string
  label: string
  pickerLabel: string
  invalid?: boolean
  describedBy?: string
  disabled?: boolean
  onChange: (value: string) => void
  onKeyDown?: KeyboardEventHandler<HTMLInputElement>
}

export function ColorInput({
  id,
  value,
  label,
  pickerLabel,
  invalid,
  describedBy: describedById,
  disabled,
  onChange,
  onKeyDown,
}: ColorInputProps) {
  const trimmed = value.trim()
  return (
    <div className="flex items-center gap-2">
      <input
        type="color"
        aria-label={pickerLabel}
        value={HEX_COLOR_PATTERN.test(trimmed) ? trimmed.toLowerCase() : FALLBACK_COLOR}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 w-10 shrink-0 cursor-pointer rounded-md border border-input bg-background p-1 disabled:cursor-not-allowed disabled:opacity-50"
      />
      <Input
        id={id}
        aria-label={label}
        value={value}
        placeholder="#1f6feb"
        maxLength={7}
        disabled={disabled}
        aria-invalid={invalid || undefined}
        aria-describedby={describedById}
        className="w-28 font-mono"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
      />
    </div>
  )
}
