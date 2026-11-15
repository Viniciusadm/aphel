import { Check } from 'lucide-react'
import { createContext, useContext } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import type { RecordItem } from '@/sdk'
import { DealCurrencyControl } from '../DealCurrencyControl'
import { useDealRunner } from './DealActions'

export const DealAmountLockContext = createContext(false)

export function DealStageStepper({ record }: { record: RecordItem }) {
  const { t } = useTranslation('deals')
  const runner = useDealRunner(record)
  const amountLocked = useContext(DealAmountLockContext)
  const { deal, allStages, currentStage } = runner
  const open = allStages.filter((stage) => stage.is_active && stage.kind === 'open').sort((left, right) => left.position - right.position)
  const currentIndex = open.findIndex((stage) => stage.id === deal.stage_id)
  if (open.length === 0 && !runner.canUpdate) return null
  const closedLabel = deal.status === 'won' ? t('detail.statusWon') : deal.status === 'lost' ? t('detail.statusLost') : null
  return (
    <div className="grid gap-3 rounded-lg border bg-card p-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <ol className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5" aria-label={t('detail.stages')}>
          {open.map((stage, index) => {
            const done = currentIndex >= 0 && index < currentIndex
            const current = index === currentIndex
            const disabled = !runner.canUpdate || runner.busy || current || deal.status !== 'open'
            return (
              <li key={stage.id}>
                <button
                  type="button"
                  disabled={disabled}
                  aria-current={current ? 'step' : undefined}
                  title={current ? undefined : t('detail.moveToStage', { stage: stage.name })}
                  onClick={() => runner.moveTo(stage)}
                  className={cn(
                    'inline-flex min-h-8 items-center gap-1.5 rounded-full border px-3 text-sm transition-colors',
                    current && 'border-primary bg-primary/10 font-medium text-foreground',
                    done && 'border-transparent bg-muted text-muted-foreground',
                    !current && !done && 'text-muted-foreground',
                    !disabled && 'hover:bg-accent',
                    disabled && !current && 'cursor-default',
                  )}
                >
                  {done && <Check className="size-3.5" aria-hidden="true" />}
                  {stage.name}
                </button>
              </li>
            )
          })}
          {closedLabel && (
            <li>
              <span className={cn('inline-flex min-h-8 items-center rounded-full border px-3 text-sm font-medium', deal.status === 'won' ? 'border-success/40 bg-success/10 text-success' : 'border-destructive/40 bg-destructive/10 text-destructive')}>
                {closedLabel}
              </span>
            </li>
          )}
        </ol>
        <DealCurrencyControl record={record} readOnly={!runner.canUpdate || amountLocked} />
      </div>
      {runner.dialogs}
    </div>
  )
}
