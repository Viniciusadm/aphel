import { ArrowRight } from 'lucide-react'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import type { ConflictDecision, PlanAction, PlanChange, PlanItem } from './api'
import { isStructured, useValueDescriber } from './lib/describe-value'
import { visibleChanges } from './lib/plan-changes'
import { usePackageLabels } from './lib/labels'

const ACTION_VARIANT: Record<PlanAction, BadgeProps['variant']> = {
  create: 'success',
  update: 'default',
  unchanged: 'outline',
  skip_conflict: 'warning',
}

export function ActionBadge({ action }: { action: PlanAction }) {
  const { t } = useTranslation('config-packages')
  return <Badge variant={ACTION_VARIANT[action]}>{t(`actions.${action}`)}</Badge>
}

interface PlanItemCardProps {
  item: PlanItem
  decision: ConflictDecision
  onDecisionChange: (decision: ConflictDecision) => void
}

export function PlanItemCard({ item, decision, onDecisionChange }: PlanItemCardProps) {
  const { t } = useTranslation('config-packages')
  const labels = usePackageLabels()
  const title = item.label || item.key
  const changes = visibleChanges(item)

  return (
    <article className="grid min-w-0 gap-3 rounded-lg border bg-card p-3 sm:p-4" aria-label={title}>
      <header className="flex min-w-0 flex-wrap items-center gap-2">
        <h4 className="min-w-0 font-medium break-words">{title}</h4>
        <Badge variant="secondary">{labels.kind(item.kind)}</Badge>
        <ActionBadge action={item.action} />
        <code className="min-w-0 text-xs break-all text-muted-foreground">{item.key}</code>
      </header>
      {changes.length > 0 && (
        <ul className="grid min-w-0 gap-2" aria-label={t('plan.changesLabel', { name: title })}>
          {changes.map((change) => (
            <ChangeRow key={change.property} change={change} creating={item.action === 'create'} />
          ))}
        </ul>
      )}
      {item.conflict && (
        <ConflictBlock
          title={title}
          message={item.conflict.message}
          resolvable={item.conflict.resolvable}
          decision={decision}
          onDecisionChange={onDecisionChange}
        />
      )}
    </article>
  )
}

function ChangeRow({ change, creating }: { change: PlanChange; creating: boolean }) {
  const { t } = useTranslation('config-packages')
  const labels = usePackageLabels()
  const describe = useValueDescriber()
  const current: unknown = change.current
  const proposed: unknown = change.proposed
  const showCurrent = !creating && current !== null && current !== undefined
  const structured = isStructured(current) || isStructured(proposed)

  return (
    <li className="grid min-w-0 gap-1 text-sm">
      <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
        <span className="font-medium text-muted-foreground">{labels.property(change.property)}</span>
        {showCurrent && (
          <>
            <span className="min-w-0 break-words text-muted-foreground line-through">
              <span className="sr-only">{t('plan.currentValue')} </span>
              {describe(current)}
            </span>
            <ArrowRight className="size-3.5 shrink-0 self-center text-muted-foreground" aria-hidden="true" />
          </>
        )}
        <span className="min-w-0 break-words">
          {showCurrent && <span className="sr-only">{t('plan.proposedValue')} </span>}
          {describe(proposed)}
        </span>
      </div>
      {structured && (
        <details className="min-w-0 text-xs">
          <summary className="cursor-pointer text-primary underline-offset-4 hover:underline">{t('plan.details')}</summary>
          <div className="mt-2 grid min-w-0 gap-2 sm:grid-cols-2">
            {showCurrent && <JsonBlock title={t('plan.currentValue')} value={current} />}
            <JsonBlock title={t('plan.proposedValue')} value={proposed} />
          </div>
        </details>
      )}
    </li>
  )
}

function JsonBlock({ title, value }: { title: string; value: unknown }) {
  return (
    <div className="grid min-w-0 gap-1">
      <span className="font-medium text-muted-foreground">{title}</span>
      <pre className="max-h-64 min-w-0 overflow-auto rounded-md bg-muted p-2 font-mono text-xs">
        {JSON.stringify(value, null, 2)}
      </pre>
    </div>
  )
}

interface ConflictBlockProps {
  title: string
  message: string
  resolvable: boolean
  decision: ConflictDecision
  onDecisionChange: (decision: ConflictDecision) => void
}

function ConflictBlock({ title, message, resolvable, decision, onDecisionChange }: ConflictBlockProps) {
  const { t } = useTranslation('config-packages')
  const name = useId()

  return (
    <div className="grid gap-2 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
      <p className="break-words">{message}</p>
      {resolvable ? (
        <fieldset className="grid gap-1.5">
          <legend className="mb-1 font-medium">{t('plan.decisionLegend', { name: title })}</legend>
          {(['skip', 'apply'] as const).map((value) => (
            <label key={value} className="flex items-center gap-2">
              <input
                type="radio"
                name={name}
                value={value}
                checked={decision === value}
                className="size-4 accent-primary"
                onChange={() => onDecisionChange(value)}
              />
              {t(`plan.decisions.${value}`)}
            </label>
          ))}
        </fieldset>
      ) : (
        <p className="font-medium">{t('plan.willSkip')}</p>
      )}
    </div>
  )
}
