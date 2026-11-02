import { ArrowRight, History } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useFieldSchema } from '@/features/records/api'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { useLifecycleHistory } from './api'
import { LIFECYCLE_STAGE_FIELD } from './constants'
import type { LifecycleRecordType } from './types'

export interface LifecycleHistoryTabProps {
  recordType: LifecycleRecordType
  recordId: string
}

interface StageLabel {
  label: string
  color: string | null
}

function useStageLabels(recordType: LifecycleRecordType): Map<string, StageLabel> {
  const schema = useFieldSchema(recordType)
  return useMemo(() => {
    const labels = new Map<string, StageLabel>()
    const field = schema.data?.fields.find((item) => item.key === LIFECYCLE_STAGE_FIELD)
    for (const option of field?.options ?? []) {
      labels.set(option.value, { label: option.label, color: option.color ?? null })
    }
    return labels
  }, [schema.data])
}

export function LifecycleHistoryTab({ recordType, recordId }: LifecycleHistoryTabProps) {
  const { t, i18n } = useTranslation('leads')
  const history = useLifecycleHistory(recordType, recordId)
  const labels = useStageLabels(recordType)
  const dateFormat = useMemo(
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium', timeStyle: 'short' }),
    [i18n.language],
  )
  const daysFormat = useMemo(
    () => new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 }),
    [i18n.language],
  )

  const stage = (value: string | null | undefined) => {
    if (!value) return <span className="text-muted-foreground">{t('history.noStage')}</span>
    const known = labels.get(value)
    return (
      <Badge variant="outline" className="gap-1.5">
        {known?.color && (
          <span aria-hidden="true" className="size-2 rounded-full" style={{ backgroundColor: known.color }} />
        )}
        {known?.label ?? value}
      </Badge>
    )
  }

  if (history.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  }
  if (history.isError) return <ErrorState onRetry={() => void history.refetch()} />
  if (history.data.length === 0) {
    return <EmptyState icon={History} title={t('history.empty')} description={t('history.emptyDescription')} />
  }

  return (
    <section className="grid gap-3" aria-label={t('history.title')}>
      <p className="text-sm text-muted-foreground">{t('history.description')}</p>
      <ol className="grid gap-2">
        {history.data.map((entry) => (
          <li key={entry.id} className="grid gap-1 rounded-md border bg-card px-3 py-2">
            <div className="flex flex-wrap items-center gap-2 text-sm">
              {stage(entry.from_stage)}
              <ArrowRight aria-hidden="true" className="size-4 text-muted-foreground" />
              {stage(entry.to_stage)}
              {entry.left_at == null && <Badge>{t('history.current')}</Badge>}
            </div>
            <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
              <span>{t('history.entered', { date: dateFormat.format(new Date(entry.entered_at)) })}</span>
              {entry.left_at && <span>{t('history.left', { date: dateFormat.format(new Date(entry.left_at)) })}</span>}
              {entry.days_in_stage != null && (
                <span>{t('history.days', { days: daysFormat.format(entry.days_in_stage) })}</span>
              )}
              <span>
                {t('history.by', {
                  actor:
                    entry.actor.name ??
                    (entry.actor.type === 'user' && entry.actor.id !== 'system'
                      ? t('history.someone')
                      : t('history.system')),
                })}
              </span>
            </div>
          </li>
        ))}
      </ol>
    </section>
  )
}

function isLifecycleRecordType(recordType: string): recordType is LifecycleRecordType {
  return recordType === 'contacts.contact' || recordType === 'companies.company'
}

export function LifecycleRecordTab({ recordType, recordId }: RecordTabRenderProps) {
  if (!isLifecycleRecordType(recordType)) return null
  return <LifecycleHistoryTab recordType={recordType} recordId={recordId} />
}
