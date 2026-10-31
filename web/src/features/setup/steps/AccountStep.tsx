import { LoaderCircle } from 'lucide-react'
import type { UseFormReturn } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { FormRow } from '@/features/settings/organization/components/FormRow'
import { SetupStepFrame } from '@/kernel/setup'
import type { SetupValues } from '../lib/form'

export interface SummaryLine {
  key: string
  label: string
  value: string
  onEdit: () => void
}

interface AccountStepProps {
  form: UseFormReturn<SetupValues>
  summary: SummaryLine[]
  error: string | null
  submitting: boolean
  progress: string | null
  onSubmit: () => void
}

export function AccountStep({ form, summary, error, submitting, progress, onSubmit }: AccountStepProps) {
  const { t } = useTranslation('setup')
  const {
    register,
    formState: { errors },
  } = form
  const message = (key?: string) => (key ? t(key as 'errors.required') : undefined)

  return (
    <SetupStepFrame
      title={t('account.title')}
      description={t('account.description')}
      error={error}
      primary={{ label: t('account.submit'), onClick: onSubmit, busy: submitting, busyLabel: t('account.submitting') }}
    >
      <fieldset className="grid gap-4" disabled={submitting}>
        <FormRow id="setup-admin-name" label={t('account.fields.adminName')} error={message(errors.adminName?.message)}>
          {(describedBy) => (
            <Input placeholder={t('placeholders.adminName')}
              id="setup-admin-name"
              autoComplete="name"
              aria-invalid={errors.adminName ? true : undefined}
              aria-describedby={describedBy}
              {...register('adminName')}
            />
          )}
        </FormRow>
        <FormRow id="setup-admin-email" label={t('account.fields.adminEmail')} error={message(errors.adminEmail?.message)}>
          {(describedBy) => (
            <Input placeholder={t('placeholders.adminEmail')}
              id="setup-admin-email"
              type="email"
              autoComplete="email"
              aria-invalid={errors.adminEmail ? true : undefined}
              aria-describedby={describedBy}
              {...register('adminEmail')}
            />
          )}
        </FormRow>
        <FormRow
          id="setup-admin-password"
          label={t('account.fields.adminPassword')}
          error={message(errors.adminPassword?.message)}
          hint={errors.adminPassword ? undefined : t('account.hints.password')}
        >
          {(describedBy) => (
            <Input placeholder={t('placeholders.adminPassword')}
              id="setup-admin-password"
              type="password"
              autoComplete="new-password"
              aria-invalid={errors.adminPassword ? true : undefined}
              aria-describedby={describedBy}
              {...register('adminPassword')}
            />
          )}
        </FormRow>
      </fieldset>
      {submitting && (
        <p role="status" className="flex items-center gap-2 rounded-md border bg-muted/30 p-3 text-sm">
          <LoaderCircle className="size-4 shrink-0 animate-spin text-primary" aria-hidden />
          {progress ?? t('account.submitting')}
        </p>
      )}
      <section className="grid gap-2 rounded-lg border bg-muted/30 p-4" aria-labelledby="setup-summary">
        <h2 id="setup-summary" className="text-sm font-semibold">
          {t('account.summary')}
        </h2>
        <dl className="grid gap-2 text-sm">
          {summary.map((line) => (
            <div key={line.key} className="flex items-start justify-between gap-3">
              <div className="grid min-w-0 gap-0.5">
                <dt className="text-xs text-muted-foreground">{line.label}</dt>
                <dd className="break-words">{line.value}</dd>
              </div>
              <Button type="button" variant="link" size="sm" className="h-auto shrink-0 p-0" disabled={submitting} onClick={line.onEdit}>
                {t('actions.edit')}
              </Button>
            </div>
          ))}
        </dl>
      </section>
    </SetupStepFrame>
  )
}
