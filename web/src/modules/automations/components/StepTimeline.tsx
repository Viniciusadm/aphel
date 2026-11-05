import { CheckCircle2, CircleSlash, Clock, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { type FieldSchema, formatDateTime, useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import { useStepSummary } from '../editor/summary'
import { useAutomationLabels } from '../lib/labels'
import type { AutomationCatalog, RunStep, StepSpec } from '../types'
import { StepStatusBadge } from './StatusBadges'

export interface StepTimelineProps {
  steps: RunStep[]
  definitionSteps: StepSpec[]
  catalog: AutomationCatalog | undefined
  schema: FieldSchema | undefined
  waitingStepId?: string | null
}

function hasContent(value: unknown): boolean {
  if (value === null || value === undefined) return false
  if (typeof value === 'object' && !Array.isArray(value)) return Object.keys(value).length > 0
  if (Array.isArray(value)) return value.length > 0
  return true
}

function JsonBlock({ label, value }: { label: string; value: unknown }) {
  if (!hasContent(value)) return null
  return (
    <details className="rounded-md border bg-muted/40">
      <summary className="cursor-pointer px-2 py-1 text-xs font-medium">{label}</summary>
      <pre className="max-h-64 overflow-auto px-2 pb-2 text-xs break-all whitespace-pre-wrap">
        {JSON.stringify(value, null, 2)}
      </pre>
    </details>
  )
}

export function StepTimeline({ steps, definitionSteps, catalog, schema, waitingStepId }: StepTimelineProps) {
  const { t } = useTranslation('automations')
  const formatting = useRecordFormatting()
  const summary = useStepSummary(schema, catalog)
  const labels = useAutomationLabels(catalog)
  const byId = new Map(definitionSteps.map((step) => [step.id, step]))
  const titleOf = (id: string) => {
    const step = byId.get(id)
    return step ? summary(step).title : id
  }
  const ordered = [...steps].sort((left, right) => left.position - right.position)

  if (ordered.length === 0) {
    return <p className="text-sm text-muted-foreground">{t('timeline.empty')}</p>
  }

  return (
    <ol className="grid gap-3">
      {ordered.map((step) => {
        const Icon =
          step.status === 'failed' ? XCircle : step.status === 'waiting' ? Clock : step.status === 'skipped' ? CircleSlash : CheckCircle2
        const definition = byId.get(step.step_id)
        return (
          <li
            key={`${step.step_id}-${step.position}`}
            className={cn(
              'grid gap-2 rounded-lg border bg-card p-3',
              step.status === 'failed' && 'border-destructive/60',
              step.step_id === waitingStepId && 'border-warning',
            )}
          >
            <div className="flex flex-wrap items-center gap-2">
              <Icon
                className={cn(
                  'size-4 shrink-0',
                  step.status === 'failed' ? 'text-destructive' : step.status === 'waiting' ? 'text-warning' : 'text-success',
                  (step.status === 'skipped' || step.status === 'planned') && 'text-muted-foreground',
                )}
                aria-hidden="true"
              />
              <span className="min-w-0 flex-1 truncate font-medium">{titleOf(step.step_id)}</span>
              <StepStatusBadge status={step.status} />
            </div>
            <dl className="grid gap-1 text-xs text-muted-foreground sm:grid-cols-2">
              <div className="flex gap-1">
                <dt>{t('timeline.kind')}</dt>
                <dd className="text-foreground">
                  {t(`steps.${step.kind}`, { defaultValue: step.kind })}
                  {step.action ? ` · ${labels.actionLabel(step.action)}` : ''}
                </dd>
              </div>
              <div className="flex gap-1">
                <dt>{t('timeline.startedAt')}</dt>
                <dd className="text-foreground tabular-nums">
                  {formatDateTime(step.started_at, formatting)} · {t('timeline.duration', { ms: step.duration_ms })}
                </dd>
              </div>
              {step.condition_result !== null && step.condition_result !== undefined && (
                <div className="flex gap-1">
                  <dt>{t('timeline.condition')}</dt>
                  <dd className={step.condition_result ? 'text-success' : 'text-foreground'}>
                    {step.condition_result ? t('timeline.conditionTrue') : t('timeline.conditionFalse')}
                  </dd>
                </div>
              )}
              {step.next_step_id && (
                <div className="flex gap-1">
                  <dt>{t('timeline.next')}</dt>
                  <dd className="text-foreground">{titleOf(step.next_step_id)}</dd>
                </div>
              )}
            </dl>
            {definition?.type === 'condition' && step.condition_result === false && (
              <p className="text-xs text-muted-foreground">{t('timeline.stoppedByCondition')}</p>
            )}
            {step.error && (
              <p role="alert" className="rounded-md bg-destructive/10 px-2 py-1 text-xs break-words text-destructive">
                {step.error}
              </p>
            )}
            <JsonBlock label={t('timeline.input')} value={step.input} />
            <JsonBlock label={t('timeline.output')} value={step.output} />
          </li>
        )
      })}
    </ol>
  )
}
