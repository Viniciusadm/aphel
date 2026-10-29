import type { ComponentProps } from 'react'
import { useTranslation } from 'react-i18next'
import { FormRow } from '@/features/settings/organization/components/FormRow'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import { CREDIT_MODES } from './api'

export interface CreditModeFieldProps extends Omit<ComponentProps<'select'>, 'id'> {
  id: string
}

export function CreditModeField({ id, className, ...props }: CreditModeFieldProps) {
  const { t } = useTranslation('goals')
  return (
    <div className={className}>
      <FormRow id={id} label={t('creditMode.label')} hint={t('creditMode.hint')}>
        {(describedBy) => (
          <NativeSelect id={id} aria-describedby={describedBy} {...props}>
            {CREDIT_MODES.map((mode) => (
              <option key={mode} value={mode}>
                {t(`creditMode.${mode}`)}
              </option>
            ))}
          </NativeSelect>
        )}
      </FormRow>
    </div>
  )
}
