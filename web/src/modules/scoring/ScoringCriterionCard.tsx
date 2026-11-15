import { ArrowDown, ArrowUp, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberMaskInput } from '@/components/ui/number-mask-input'
import { FilterBuilder, NativeSelect, SettingsCard, WIDE_FIELD, type FieldSchema } from '@/sdk'
import { LABEL_MAX_LENGTH, type CriterionDraft } from './scoring-form'
import { SCORING_SIGNALS, type ScoringSignal } from './types'

interface ScoringCriterionCardProps {
  criterion: CriterionDraft
  index: number
  total: number
  schema: FieldSchema | undefined
  disabled: boolean
  error: (field: string) => string | undefined
  onChange: (next: Partial<CriterionDraft>) => void
  onMove: (offset: number) => void
  onRemove: () => void
}

export function ScoringCriterionCard({
  criterion,
  index,
  total,
  schema,
  disabled,
  error,
  onChange,
  onMove,
  onRemove,
}: ScoringCriterionCardProps) {
  const { t } = useTranslation('scoring')
  const path = (field: string) => `criteria.${index}.${field}`
  const name = criterion.label || t('dialog.unnamedCriterion')

  return (
    <div className="grid min-w-0 gap-2">
      <div className="flex min-w-0 items-center justify-between gap-2">
        <h4 className="min-w-0 truncate text-sm font-medium">
          {index + 1}. {name}
        </h4>
        <div className="flex items-center gap-1">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            disabled={disabled || index === 0}
            aria-label={t('dialog.moveUp', { label: name })}
            onClick={() => onMove(-1)}
          >
            <ArrowUp aria-hidden="true" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            disabled={disabled || index === total - 1}
            aria-label={t('dialog.moveDown', { label: name })}
            onClick={() => onMove(1)}
          >
            <ArrowDown aria-hidden="true" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="text-destructive"
            disabled={disabled}
            aria-label={t('dialog.removeCriterion', { label: name })}
            onClick={onRemove}
          >
            <Trash2 aria-hidden="true" />
          </Button>
        </div>
      </div>
      <SettingsCard title={t(`kinds.${criterion.kind}`)} description={t(`dialog.kindHints.${criterion.kind}`)} disabled={disabled}>
        <Field label={t('dialog.criterionLabel')} error={error(path('label'))} className={criterion.kind === 'behavior' ? WIDE_FIELD : undefined}>
          {(control) => (
            <Input
              {...control}
              value={criterion.label}
              maxLength={LABEL_MAX_LENGTH}
              placeholder={t('placeholders.criterionLabel')}
              onChange={(event) => onChange({ label: event.target.value })}
            />
          )}
        </Field>
        <Field label={t('dialog.points')} description={t('dialog.pointsHint')} error={error(path('points'))}>
          {(control) => (
            <NumberMaskInput
              {...control}
              kind="signedInteger"
              maxLength={5}
              className="w-32"
              placeholder={t('placeholders.points')}
              value={criterion.points}
              onValueChange={(points) => onChange({ points })}
            />
          )}
        </Field>
        {criterion.kind === 'profile' ? (
          <Field label={t('dialog.filter')} error={error(path('filter'))} className={WIDE_FIELD}>
            {() => schema && <FilterBuilder schema={schema} value={criterion.filter} onChange={(filter) => onChange({ filter })} />}
          </Field>
        ) : (
          <>
            <Field label={t('dialog.signal')} error={error(path('signal'))}>
              {(control) => (
                <NativeSelect
                  {...control}
                  value={criterion.signal}
                  onChange={(event) => onChange({ signal: event.target.value as ScoringSignal })}
                >
                  {SCORING_SIGNALS.map((signal) => (
                    <option key={signal} value={signal}>
                      {t(`signals.${signal}`)}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </Field>
            <Field label={t('dialog.maxPoints')} description={t('dialog.maxPointsHint')} error={error(path('max_points'))}>
              {(control) => (
                <NumberMaskInput
                  {...control}
                  kind="integer"
                  maxLength={6}
                  className="w-32"
                  placeholder={t('placeholders.maxPoints')}
                  value={criterion.maxPoints}
                  onValueChange={(maxPoints) => onChange({ maxPoints })}
                />
              )}
            </Field>
            <Field label={t('dialog.decayDays')} description={t('dialog.decayHint')} error={error(path('decay_days'))}>
              {(control) => (
                <NumberMaskInput
                  {...control}
                  kind="integer"
                  maxLength={4}
                  className="w-32"
                  placeholder={t('placeholders.decayDays')}
                  value={criterion.decayDays}
                  onValueChange={(decayDays) => onChange({ decayDays })}
                />
              )}
            </Field>
          </>
        )}
      </SettingsCard>
    </div>
  )
}
