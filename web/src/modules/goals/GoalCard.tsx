import { Building2, Pencil, Trash2, User, Users } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import type { Goal, GoalStatus } from './api'
import { formatGoalValue, useGoalPeriodLabel } from './format'

const STATUS_VARIANT: Record<GoalStatus, BadgeProps['variant']> = {
  upcoming: 'outline',
  on_track: 'secondary',
  behind: 'warning',
  achieved: 'success',
  missed: 'destructive',
}

const BAR_COLOR: Record<GoalStatus, string> = {
  upcoming: 'bg-muted-foreground/40',
  on_track: 'bg-primary',
  behind: 'bg-warning',
  achieved: 'bg-success',
  missed: 'bg-destructive',
}

function OwnerIcon({ goal }: { goal: Goal }) {
  const Icon = goal.owner_type === 'organization' ? Building2 : goal.owner_type === 'team' ? Users : User
  return <Icon className="size-4 shrink-0 text-muted-foreground" aria-hidden />
}

export function GoalProgressBar({ goal, className }: { goal: Goal; className?: string }) {
  const { t } = useTranslation('goals')
  const progress = goal.progress
  if (!progress) {
    return <p className={cn('text-xs text-muted-foreground', className)}>{t('progress.unavailable')}</p>
  }
  const percent = Math.round(progress.ratio * 100)
  const width = Math.min(100, Math.max(0, progress.ratio * 100))
  const expected = Math.min(100, Math.max(0, progress.expected_ratio * 100))
  return (
    <div className={cn('grid gap-1', className)}>
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.min(100, percent)}
        aria-valuetext={t('progress.valueText', { percent })}
        className="relative h-2.5 w-full overflow-hidden rounded-full bg-muted"
      >
        <div className={cn('h-full rounded-full transition-all', BAR_COLOR[progress.status])} style={{ width: `${width}%` }} />
        {progress.status !== 'upcoming' && expected > 0 && expected < 100 && (
          <span
            aria-hidden
            className="absolute top-0 h-full w-0.5 bg-foreground/60"
            style={{ left: `${expected}%` }}
            title={t('progress.expected', { percent: Math.round(expected) })}
          />
        )}
      </div>
      <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
        <span className="tabular-nums">{t('progress.percent', { percent })}</span>
        {progress.status !== 'upcoming' && progress.status !== 'achieved' && progress.status !== 'missed' && (
          <span className="truncate">{t('progress.expected', { percent: Math.round(expected) })}</span>
        )}
      </div>
    </div>
  )
}

export function GoalStatusBadge({ status }: { status: GoalStatus }) {
  const { t } = useTranslation('goals')
  return <Badge variant={STATUS_VARIANT[status]}>{t(`status.${status}`)}</Badge>
}

interface GoalCardProps {
  goal: Goal
  compact?: boolean
  onEdit?: (goal: Goal) => void
  onDelete?: (goal: Goal) => void
}

export function GoalCard({ goal, compact = false, onEdit, onDelete }: GoalCardProps) {
  const { t } = useTranslation('goals')
  const formatting = useRecordFormatting()
  const periodLabel = useGoalPeriodLabel()
  const ownerName =
    goal.owner_type === 'organization' ? t('owner.organization') : (goal.owner_name ?? t('owner.unknown'))
  const metricLabel = t(`metrics.${goal.metric}`)
  const actual = goal.progress?.actual
  const target = formatGoalValue(goal.metric, goal.target, formatting, goal.currency)
  return (
    <article
      className={cn('flex min-w-0 flex-col gap-3 rounded-lg border bg-card', compact ? 'p-3' : 'p-4')}
      aria-label={t('card.label', { metric: metricLabel, owner: ownerName })}
    >
      <div className="flex min-w-0 items-start justify-between gap-2">
        <div className="grid min-w-0 gap-0.5">
          <p className="truncate font-medium">{metricLabel}</p>
          <p className="flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground">
            <OwnerIcon goal={goal} />
            <span className="truncate">{ownerName}</span>
            <span aria-hidden>·</span>
            <span className="truncate">{periodLabel(goal)}</span>
          </p>
        </div>
        {goal.progress && <GoalStatusBadge status={goal.progress.status} />}
      </div>
      <div className="flex min-w-0 flex-wrap items-baseline justify-between gap-x-2 gap-y-1">
        <span className="text-lg font-semibold tabular-nums">
          {actual !== undefined ? formatGoalValue(goal.metric, actual, formatting, goal.currency) : t('progress.noValue')}
        </span>
        <span className="text-sm text-muted-foreground tabular-nums">{t('card.target', { target })}</span>
      </div>
      <GoalProgressBar goal={goal} />
      {goal.can_manage && (onEdit || onDelete) && (
        <div className="flex justify-end gap-1">
          {onEdit && (
            <Button size="sm" variant="ghost" aria-label={t('card.edit', { metric: metricLabel, owner: ownerName })} onClick={() => onEdit(goal)}>
              <Pencil aria-hidden="true" />
            </Button>
          )}
          {onDelete && (
            <Button
              size="sm"
              variant="ghost"
              aria-label={t('card.delete', { metric: metricLabel, owner: ownerName })}
              onClick={() => onDelete(goal)}
            >
              <Trash2 aria-hidden="true" />
            </Button>
          )}
        </div>
      )}
    </article>
  )
}
