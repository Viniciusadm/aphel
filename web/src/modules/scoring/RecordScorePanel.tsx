import { Gauge } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { PageLoader } from '@/components/feedback'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { useRecordScores } from './api'
import type { RecordScore, ScoreBand } from './types'

const BAND_VARIANTS: Record<ScoreBand, BadgeProps['variant']> = {
  cold: 'secondary',
  warm: 'warning',
  hot: 'destructive',
}

export interface RecordScorePanelProps {
  recordType: string
  recordId: string
}

export function RecordScorePanel({ recordType, recordId }: RecordScorePanelProps) {
  const { t } = useTranslation('scoring')
  const scores = useRecordScores(recordType, recordId)

  if (scores.isPending) return <PageLoader />
  if (scores.isError) return <ErrorState onRetry={() => void scores.refetch()} />
  if (scores.data.length === 0) return <EmptyState icon={Gauge} title={t('record.empty')} />

  return (
    <ul className="grid gap-3" aria-label={t('record.tab')}>
      {scores.data.map((score) => (
        <ScoreCard key={score.model_id} score={score} />
      ))}
    </ul>
  )
}

function ScoreCard({ score }: { score: RecordScore }) {
  const { t, i18n } = useTranslation('scoring')
  const dateFormat = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' })
  const titleId = `score-${score.model_id}`

  return (
    <li className="grid min-w-0 gap-3 rounded-md border bg-card p-4" aria-labelledby={titleId}>
      <div className="flex min-w-0 flex-wrap items-start justify-between gap-3">
        <div className="grid min-w-0 gap-1">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <h3 id={titleId} className="font-medium break-words">
              {score.model_name}
            </h3>
            {score.is_primary && <Badge variant="outline">{t('record.primary')}</Badge>}
          </div>
          <p className="text-xs text-muted-foreground">
            {t('record.thresholds', { warm: score.warm_from, hot: score.hot_from })}
          </p>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-3xl font-semibold tabular-nums" aria-label={`${t('record.score')}: ${score.score}`}>
            {score.score}
          </span>
          <Badge variant={BAND_VARIANTS[score.band]}>{t(`bands.${score.band}`)}</Badge>
        </div>
      </div>
      <div className="grid gap-2">
        <h4 className="text-sm font-medium">{t('record.explanationTitle')}</h4>
        {score.explanation.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('record.noContributions')}</p>
        ) : (
          <ul className="grid gap-1">
            {score.explanation.map((item) => (
              <li
                key={item.criterion_id}
                className="flex min-w-0 items-center justify-between gap-3 rounded border px-3 py-2 text-sm"
              >
                <span className="grid min-w-0 gap-0.5">
                  <span className="break-words">{item.label}</span>
                  <span className="text-xs text-muted-foreground">
                    {t(`kinds.${item.kind}`)}
                    {item.kind === 'behavior' && ` · ${t('record.occurrences', { count: item.occurrences })}`}
                  </span>
                </span>
                <span
                  className={
                    item.points >= 0
                      ? 'shrink-0 font-medium text-success tabular-nums'
                      : 'shrink-0 font-medium text-destructive tabular-nums'
                  }
                >
                  {item.points > 0 ? `+${item.points}` : item.points}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
      <p className="text-xs text-muted-foreground">
        {score.calculated_at
          ? t('record.calculatedAt', { date: dateFormat.format(new Date(score.calculated_at)) })
          : t('record.notCalculated')}
      </p>
    </li>
  )
}
