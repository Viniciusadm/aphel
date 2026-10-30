import { useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useTemplates } from '../api'
import { SCRATCH } from '../lib/templates'
import { rankTemplates } from '../lib/templates'
import type { OnboardingRecommendation } from '../types'
import { OptionCard } from '@/components/ui/option-card'

interface RecommendationStepProps {
  recommendation: OnboardingRecommendation | null
  selected: string | null
  onSelect: (key: string) => void
  disabled?: boolean
}

export function RecommendationStep({ recommendation, selected, onSelect, disabled }: RecommendationStepProps) {
  const { t } = useTranslation('onboarding')
  const templates = useTemplates()
  const options = useMemo(
    () => rankTemplates(templates.data ?? [], recommendation),
    [templates.data, recommendation],
  )

  const fallback = templates.isSuccess ? (options[0]?.template.key ?? SCRATCH) : null

  useEffect(() => {
    if (selected === null && fallback !== null) onSelect(fallback)
  }, [fallback, onSelect, selected])

  if (templates.isPending) {
    return (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
      </div>
    )
  }
  if (templates.isError) {
    return <ErrorState description={t('errors.templates')} onRetry={() => void templates.refetch()} />
  }

  const current = selected ?? fallback

  return (
    <fieldset className="grid gap-3" disabled={disabled}>
      <legend className="sr-only">{t('recommendation.choiceLabel')}</legend>
      {options.map(({ template, reasons, recommended }) => (
        <OptionCard
          key={template.key}
          type="radio"
          name="onboarding-template"
          value={template.key}
          checked={current === template.key}
          onChange={() => onSelect(template.key)}
          title={template.name}
          description={template.description ?? undefined}
          badge={recommended ? <Badge>{t('recommendation.recommended')}</Badge> : undefined}
        >
          {recommended && reasons.length > 0 && (
            <ul className="mt-1 flex flex-wrap gap-1.5" aria-label={t('recommendation.reasonsLabel')}>
              {reasons.map((reason) => (
                <li key={reason}>
                  <Badge variant="secondary">
                    {t(`reasons.${reason}` as 'reasons.team_solo', { defaultValue: reason })}
                  </Badge>
                </li>
              ))}
            </ul>
          )}
        </OptionCard>
      ))}
      <OptionCard
        type="radio"
        name="onboarding-template"
        value={SCRATCH}
        checked={current === SCRATCH}
        onChange={() => onSelect(SCRATCH)}
        title={t('recommendation.scratch.title')}
        description={t('recommendation.scratch.description')}
      />
    </fieldset>
  )
}
