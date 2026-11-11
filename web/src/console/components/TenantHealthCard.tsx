import { useQuery } from '@tanstack/react-query'
import { RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { consoleApi, consoleKeys } from '../api'
import { formatAge, formatDateTime } from '../format'

function Metric({ label, value, alert }: { label: string; value: number; alert?: boolean }) {
  return (
    <div className="rounded-md border border-border p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className={cn('text-xl font-semibold tabular-nums', alert && value > 0 && 'text-destructive')}>{value}</div>
    </div>
  )
}

export function TenantHealthCard({ slug, active }: { slug: string; active: boolean }) {
  const { t } = useTranslation('console')
  const health = useQuery({
    queryKey: consoleKeys.health(slug),
    queryFn: () => consoleApi.health(slug),
    enabled: active,
    refetchInterval: 30_000,
  })

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-2">
        <CardTitle>{t('health.title')}</CardTitle>
        {active && (
          <Button variant="ghost" size="sm" onClick={() => void health.refetch()} disabled={health.isFetching}>
            <RefreshCw className={cn('size-4', health.isFetching && 'animate-spin')} aria-hidden />
            {t('health.refresh')}
          </Button>
        )}
      </CardHeader>
      <CardContent className="grid gap-4">
        {!active && <p className="text-sm text-muted-foreground">{t('health.unavailable')}</p>}
        {active && health.isPending && <Skeleton className="h-32 w-full" />}
        {active && health.data?.error && (
          <p className="text-sm text-destructive">{t('health.error', { error: health.data.error })}</p>
        )}
        {active && health.data?.available && (
          <>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
              <Metric label={t('health.ready')} value={health.data.queue.ready} />
              <Metric label={t('health.scheduled')} value={health.data.queue.scheduled} />
              <Metric label={t('health.running')} value={health.data.queue.running} />
              <Metric label={t('health.failed')} value={health.data.queue.failed} alert />
            </div>
            {health.data.oldest_ready_age_seconds != null && health.data.oldest_ready_age_seconds > 60 && (
              <p className="text-sm text-warning">{t('health.oldest', { age: formatAge(health.data.oldest_ready_age_seconds) })}</p>
            )}
            <dl className="grid gap-3 text-sm sm:grid-cols-2">
              <div>
                <dt className="text-muted-foreground">{t('health.workers')}</dt>
                <dd>
                  {health.data.workers.length === 0 ? (
                    <span className="text-destructive">{t('health.noWorkers')}</span>
                  ) : (
                    <ul className="grid gap-1">
                      {health.data.workers.map((worker) => (
                        <li key={worker.instance_id}>
                          {t('health.workerSeen', {
                            name: worker.hostname ?? worker.instance_id,
                            when: formatDateTime(worker.last_seen_at),
                          })}
                        </li>
                      ))}
                    </ul>
                  )}
                </dd>
              </div>
              <div>
                <dt className="text-muted-foreground">{t('health.migrations')}</dt>
                <dd className={cn('tabular-nums', health.data.pending_migrations > 0 && 'text-warning')}>{health.data.pending_migrations}</dd>
              </div>
              <div>
                <dt className="text-muted-foreground">{t('health.plugins')}</dt>
                <dd className={cn(health.data.plugin_backend_errors.length > 0 && 'text-destructive')}>
                  {health.data.plugin_backend_errors.length === 0 ? t('health.none') : health.data.plugin_backend_errors.join(', ')}
                </dd>
              </div>
              <div>
                <dt className="text-muted-foreground">{t('health.failures')}</dt>
                <dd className={cn('tabular-nums', health.data.recent_failures > 0 && 'text-warning')}>{health.data.recent_failures}</dd>
              </div>
            </dl>
          </>
        )}
      </CardContent>
    </Card>
  )
}
