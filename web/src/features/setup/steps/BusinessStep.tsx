import { useTranslation } from 'react-i18next'
import { OptionCard } from '@/components/ui/option-card'
import { SetupStepFrame } from '@/kernel/setup'
import { BUSINESS_TYPES, OFFERINGS, TEAM_SIZES, type BusinessAnswers } from '../lib/business'

interface BusinessStepProps {
  value: BusinessAnswers
  onChange: (value: BusinessAnswers) => void
  error: string | null
  onNext: () => void
  onBlank: () => void
}

export function BusinessStep({ value, onChange, error, onNext, onBlank }: BusinessStepProps) {
  const { t } = useTranslation('setup')
  const legend = 'text-sm font-semibold'

  return (
    <SetupStepFrame
      title={t('business.title')}
      description={t('business.description')}
      error={error}
      primary={{ label: t('actions.next'), onClick: onNext }}
      secondary={{ label: t('business.blank'), onClick: onBlank }}
    >
      <fieldset className="grid gap-3">
        <legend className={`${legend} mb-3`}>{t('business.typeLegend')}</legend>
        <div className="grid gap-2 sm:grid-cols-2">
          {BUSINESS_TYPES.map((type) => (
            <OptionCard
              key={type}
              type="radio"
              name="setup-business-type"
              value={type}
              checked={value.business_type === type}
              onChange={() => onChange({ ...value, business_type: type })}
              title={t(`business.types.${type}.label`)}
              description={t(`business.types.${type}.description`)}
            />
          ))}
        </div>
      </fieldset>
      <fieldset className="grid gap-3">
        <legend className={`${legend} mb-1`}>
          {t('business.offeringLegend')} <span className="font-normal text-muted-foreground">· {t('business.optional')}</span>
        </legend>
        <p className="text-xs text-muted-foreground">{t('business.offeringHint')}</p>
        <div className="grid gap-2 sm:grid-cols-3">
          {OFFERINGS.map((offering) => (
            <OptionCard
              key={offering}
              type="checkbox"
              name="setup-offering"
              value={offering}
              checked={value.offering.includes(offering)}
              onChange={(checked) =>
                onChange({
                  ...value,
                  offering: checked
                    ? OFFERINGS.filter((item) => item === offering || value.offering.includes(item))
                    : value.offering.filter((item) => item !== offering),
                })
              }
              title={t(`business.offerings.${offering}.label`)}
              description={t(`business.offerings.${offering}.description`)}
            />
          ))}
        </div>
      </fieldset>
      <fieldset className="grid gap-3">
        <legend className={`${legend} mb-3`}>
          {t('business.teamLegend')} <span className="font-normal text-muted-foreground">· {t('business.optional')}</span>
        </legend>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {TEAM_SIZES.map((size) => (
            <OptionCard
              key={size}
              type="radio"
              name="setup-team-size"
              value={size}
              checked={value.team_size === size}
              onChange={() => onChange({ ...value, team_size: size })}
              title={t(`business.teamSizes.${size}`)}
            />
          ))}
        </div>
      </fieldset>
    </SetupStepFrame>
  )
}
