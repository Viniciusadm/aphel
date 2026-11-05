import { Flame } from 'lucide-react'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_PRO_KEY, MAX_TEMPERATURE, useLeadTemperature } from './api'
import { TemperatureBadge } from './TemperatureBadge'

export function TemperatureTab({ recordId }: RecordTabRenderProps) {
  const t = useModuleTranslation(LEAD_PRO_KEY)
  const formatting = useRecordFormatting()
  const query = useLeadTemperature(recordId)

  if (query.isPending) return <Skeleton className="h-40 w-full" />
  if (query.isError) return <ErrorState onRetry={() => void query.refetch()} />

  const { temperature, hot_threshold: hotThreshold, touches } = query.data
  const percent = Math.round(((temperature ?? 0) / MAX_TEMPERATURE) * 100)

  return (
    <section className="grid gap-4" aria-label={t('tab')}>
      <div className="grid gap-2 rounded-md border bg-card p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold">{t('temperature.title')}</span>
          {temperature !== null && <TemperatureBadge temperature={temperature} hotThreshold={hotThreshold} />}
        </div>
        <div
          className="h-2 overflow-hidden rounded-full bg-muted"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={MAX_TEMPERATURE}
          aria-valuenow={temperature ?? 0}
          aria-label={t('temperature.title')}
        >
          <div className="h-full bg-primary transition-all" style={{ width: `${percent}%` }} />
        </div>
        <p className="text-xs text-muted-foreground">{t('temperature.threshold', { threshold: hotThreshold })}</p>
      </div>
      <div className="grid gap-2">
        <h3 className="text-sm font-semibold">{t('touches.title')}</h3>
        {touches.length === 0 ? (
          <EmptyState icon={Flame} title={t('touches.empty')} description={t('touches.emptyDescription')} />
        ) : (
          <ul className="grid gap-1">
            {touches.map((touch) => (
              <li key={touch.id} className="flex justify-between gap-2 rounded-md border px-3 py-2 text-sm">
                <span>{t(`touches.kinds.${touch.kind.replaceAll('.', '_')}`, { defaultValue: touch.kind })}</span>
                <span className="text-muted-foreground">{formatDateTime(touch.created_at, formatting)}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}
