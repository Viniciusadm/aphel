import { useTranslation } from 'react-i18next'
import { FieldInput } from '@/components/fields/FieldInput'
import { Label } from '@/components/ui/label'
import { cn } from '@/lib/utils'
import type { FieldDefinition } from '../types'

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
  const label = (
    <Label htmlFor={id} className="font-medium">
      <span>{field.label}</span>
      {field.is_required && (
        <>
          <span aria-hidden="true" className="text-destructive">
            *
          </span>
          <span className="sr-only">({t('form.required')})</span>
        </>
      )}
    </Label>
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

  return (
    <div className={cn('grid min-w-0 content-start gap-2', className)} data-field={field.key}>
      {field.field_type === 'checkbox' ? (
        <div className="flex min-h-9 items-center gap-2">
          {input}
          {label}
        </div>
      ) : (
        <>
          {label}
          {input}
        </>
      )}
      {error && (
        <p id={errorId} className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  )
}
