import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { FieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Skeleton } from '@/components/ui/skeleton'
import { findField, useFieldSchema } from '@/sdk'
import { DEAL_RECORD_TYPE } from './resource'
import { dealFieldValue, isFilled } from './requirements'
import type { Deal } from './types'
import { Field } from '@/components/ui/field'

export interface StageRequirementsDialogProps {
  open: boolean
  deal?: Deal
  stageName?: string
  fields: string[]
  inaccessible?: boolean
  pending?: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: (values: Record<string, unknown>) => void
}

function RequirementsForm({
  deal,
  stageName,
  fields,
  pending,
  onOpenChange,
  onConfirm,
}: Omit<StageRequirementsDialogProps, 'open'>) {
  const { t } = useTranslation('deals', { keyPrefix: 'pipeline' })
  const schema = useFieldSchema(DEAL_RECORD_TYPE)
  const baseId = useId()
  const [values, setValues] = useState<Record<string, unknown>>(() =>
    Object.fromEntries(fields.map((key) => [key, deal ? (dealFieldValue(deal, key) ?? null) : null])),
  )
  const [submitted, setSubmitted] = useState(false)
  const definitions = schema.data ? fields.map((key) => ({ key, definition: findField(schema.data, key) })) : []
  const missing = fields.filter((key) => !isFilled(values[key]))

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        setSubmitted(true)
        if (missing.length === 0) onConfirm(values)
      }}
    >
      <DialogHeader>
        <DialogTitle>{t('requirements.title')}</DialogTitle>
        <DialogDescription>
          {stageName ? t('requirements.descriptionFor', { stage: stageName }) : t('requirements.description')}
        </DialogDescription>
      </DialogHeader>
      {schema.isPending ? (
        <Skeleton className="h-24 w-full" />
      ) : (
        <div className="grid gap-3">
          {definitions.map(({ key, definition }) => {
            const id = `${baseId}-${key}`
            const errorId = `${id}-error`
            const invalid = submitted && missing.includes(key)
            return (
              <Field key={key} id={id} label={definition?.label ?? key} data-field={key}>
                {definition ? (
                  <FieldInput
                    field={definition}
                    id={id}
                    value={values[key]}
                    invalid={invalid}
                    aria-describedby={invalid ? errorId : undefined}
                    onChange={(value) => setValues((current) => ({ ...current, [key]: value }))}
                  />
                ) : (
                  <p className="text-sm text-muted-foreground">{t('requirements.unknownField')}</p>
                )}
                {invalid && (
                  <SharedFieldError id={errorId}>
                    {t('requirements.required')}
                  </SharedFieldError>
                )}
              </Field>
            )
          })}
        </div>
      )}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={pending || schema.isPending}>
          {t('requirements.confirm')}
        </Button>
      </DialogFooter>
    </form>
  )
}

function InaccessibleRequirements({
  fields,
  onOpenChange,
}: Pick<StageRequirementsDialogProps, 'fields' | 'onOpenChange'>) {
  const { t } = useTranslation('deals', { keyPrefix: 'pipeline' })
  const schema = useFieldSchema(DEAL_RECORD_TYPE)
  const labels = fields.map((key) => (schema.data ? findField(schema.data, key)?.label : undefined) ?? key)

  return (
    <div className="grid gap-4">
      <DialogHeader>
        <DialogTitle>{t('requirements.title')}</DialogTitle>
        <DialogDescription>{t('requirements.inaccessible')}</DialogDescription>
      </DialogHeader>
      {labels.length > 0 && (
        <ul className="list-disc space-y-1 pl-5 text-sm">
          {labels.map((label, index) => (
            <li key={fields[index]}>{label}</li>
          ))}
        </ul>
      )}
      <DialogFooter>
        <Button type="button" onClick={() => onOpenChange(false)}>
          {t('requirements.close')}
        </Button>
      </DialogFooter>
    </div>
  )
}

export function StageRequirementsDialog({ open, onOpenChange, inaccessible, ...props }: StageRequirementsDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {open &&
          (inaccessible ? (
            <InaccessibleRequirements fields={props.fields} onOpenChange={onOpenChange} />
          ) : (
            <RequirementsForm {...props} onOpenChange={onOpenChange} />
          ))}
      </DialogContent>
    </Dialog>
  )
}
