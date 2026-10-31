import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useTranslation } from 'react-i18next'
import { FieldInput } from '@/components/fields/FieldInput'
import { Label } from '@/components/ui/label'
import { cn } from '@/lib/utils'
import type { FieldDefinition } from '../types'
import { Field } from '@/components/ui/field'

export interface RecordFieldControlProps {
  field: FieldDefinition
  id: string
  value: unknown
  onChange: (value: unknown) => void
  onBlur?: () => void
  initialValue?: unknown
  error?: string
  disabled?: boolean
  className?: string
}

export function RecordFieldControl({
  field,
  id,
  value,
  onChange,
  onBlur,
  initialValue,
  error,
  disabled,
  className,
}: RecordFieldControlProps) {
  const { t } = useTranslation('records-form')
  const errorId = `${id}-error`
  const labelContent = (
    <>
      <span>{field.label}</span>
      {field.is_required && (
        <>
          <span aria-hidden="true" className="text-destructive">
            *
          </span>
          <span className="sr-only">({t('form.required')})</span>
        </>
      )}
    </>
  )
  const input = (
    <FieldInput
      field={field}
      id={id}
      value={value}
      onChange={onChange}
      onBlur={onBlur}
      initialValue={initialValue}
      disabled={disabled}
      invalid={Boolean(error)}
      aria-describedby={error ? errorId : undefined}
    />
  )

  if (field.field_type === 'checkbox') {
    return (
      <div className={cn('grid min-w-0 content-start gap-2', className)} data-field={field.key}>
        <div className="flex min-h-9 items-center gap-2">
          {input}
          <Label htmlFor={id} className="font-medium">
            {labelContent}
          </Label>
        </div>
        {error && (
          <SharedFieldError id={errorId}>
            {error}
          </SharedFieldError>
        )}
      </div>
    )
  }

  return (
    <Field
      id={id}
      label={labelContent}
      error={error}
      data-field={field.key}
      className={cn('min-w-0 content-start', className)}
    >
      {input}
    </Field>
  )
}
