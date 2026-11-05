import { useMemo } from 'react'
import type { UseFormReturn } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { FormRow } from '@/features/settings/organization/components/FormRow'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import {
  DEFAULT_CURRENCY,
  DEFAULT_TIMEZONE,
  LOCALES,
  currencyLabel,
  currencyOptions,
  timezoneOptions,
} from '@/features/settings/organization/lib/preferences'
import { switchInstallerLanguage } from '@/kernel/modules/i18n'
import { SetupStepFrame } from '@/kernel/setup'
import type { SetupValues } from '../lib/form'

interface OrganizationStepProps {
  form: UseFormReturn<SetupValues>
  error: string | null
  onNext: () => void
  onRestore: () => void
}

export function OrganizationStep({ form, error, onNext, onRestore }: OrganizationStepProps) {
  const { t, i18n } = useTranslation('setup')
  const {
    register,
    formState: { errors },
  } = form
  const timezones = useMemo(() => timezoneOptions(DEFAULT_TIMEZONE), [])
  const currencies = useMemo(() => currencyOptions(DEFAULT_CURRENCY), [])
  const message = (key?: string) => (key ? t(key as 'errors.required') : undefined)
  const locale = register('locale')

  return (
    <SetupStepFrame
      title={t('organization.title')}
      description={t('organization.description')}
      error={error}
      primary={{ label: t('actions.next'), onClick: onNext }}
      aside={
        <button
          type="button"
          className="text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
          onClick={onRestore}
        >
          {t('organization.restore')}
        </button>
      }
    >
      <FormRow id="setup-locale" label={t('organization.fields.locale')} hint={t('organization.hints.locale')} error={message(errors.locale?.message)}>
        {(describedBy) => (
          <NativeSelect
            id="setup-locale"
            aria-describedby={describedBy}
            {...locale}
            onChange={(event) => {
              void locale.onChange(event)
              void switchInstallerLanguage(event.target.value)
            }}
          >
            {LOCALES.map((value) => (
              <option key={value} value={value}>
                {t(`locales.${value}`)}
              </option>
            ))}
          </NativeSelect>
        )}
      </FormRow>
      <FormRow id="setup-organization-name" label={t('organization.fields.organizationName')} error={message(errors.organizationName?.message)}>
        {(describedBy) => (
          <Input placeholder={t('placeholders.organizationName')}
            id="setup-organization-name"
            autoComplete="organization"
            aria-invalid={errors.organizationName ? true : undefined}
            aria-describedby={describedBy}
            {...register('organizationName')}
          />
        )}
      </FormRow>
      <div className="grid gap-4 sm:grid-cols-2">
        <FormRow id="setup-timezone" label={t('organization.fields.timezone')} hint={t('organization.hints.timezone')} error={message(errors.timezone?.message)}>
          {(describedBy) => (
            <NativeSelect id="setup-timezone" aria-describedby={describedBy} {...register('timezone')}>
              {timezones.map((zone) => (
                <option key={zone} value={zone}>
                  {zone.replaceAll('_', ' ')}
                </option>
              ))}
            </NativeSelect>
          )}
        </FormRow>
        <FormRow id="setup-currency" label={t('organization.fields.currency')} hint={t('organization.hints.currency')} error={message(errors.currency?.message)}>
          {(describedBy) => (
            <NativeSelect id="setup-currency" aria-describedby={describedBy} {...register('currency')}>
              {currencies.map((code) => (
                <option key={code} value={code}>
                  {currencyLabel(code, i18n.language)}
                </option>
              ))}
            </NativeSelect>
          )}
        </FormRow>
      </div>
    </SetupStepFrame>
  )
}
