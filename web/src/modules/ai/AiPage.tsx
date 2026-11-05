import { NativeSelect } from '@/components/ui/native-select'
import { ChevronLeft, ChevronRight, Lock, Sparkles } from 'lucide-react'
import { useState } from 'react'
import { useSession } from '@/app/session/session-context'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useOrganizationUsers, useRecordFormatting } from '@/sdk'
import { activeLocale } from '@/lib/locale'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {
  AI_FEATURES,
  AI_KEY,
  CALLS_PER_PAGE,
  USE_PERMISSION,
  VIEW_USAGE_PERMISSION,
  useAiCalls,
  useAiStatus,
  type AiCallFilters,
  type AiFeature,
  type AiStatus,
} from './api'
import { DataDisclosure } from './DataDisclosure'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const ALL = 'all'

export function AiPage() {
  const t = useModuleTranslation(AI_KEY)
  const { hasPermission } = useSession()
  const canUse = hasPermission(USE_PERMISSION)
  const canViewUsage = hasPermission(VIEW_USAGE_PERMISSION)
  const status = useAiStatus(canUse || canViewUsage)

  if (!canUse && !canViewUsage) {
    return <EmptyState icon={Lock} title={t('page.noAccessTitle')} description={t('page.noAccessDescription')} />
  }

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      {status.isPending ? (
        <Skeleton className="h-40 w-full" />
      ) : status.isError ? (
        <ErrorState onRetry={() => void status.refetch()} />
      ) : (
        <>
          <StatusCard status={status.data} />
          <DataDisclosure status={status.data} />
        </>
      )}
      {canViewUsage && <CallLog />}
    </Page>
  )
}

function StatusCard({ status }: { status: AiStatus }) {
  const t = useModuleTranslation(AI_KEY)
  const percent = status.daily_limit > 0 ? Math.min(100, Math.round((status.used_today / status.daily_limit) * 100)) : 0
  return (
    <div className="grid gap-3 rounded-md border bg-card p-4">
      <div className="flex flex-wrap items-center gap-2">
        <Sparkles className="size-4 text-muted-foreground" aria-hidden="true" />
        <span className="text-sm font-semibold">{t('page.status')}</span>
        <Badge variant={status.configured ? 'success' : 'warning'}>
          {t(status.configured ? 'page.configured' : 'page.notConfigured')}
        </Badge>
      </div>
      {!status.configured && <p className="text-sm text-muted-foreground">{t('page.notConfiguredHelp')}</p>}
      <div className="grid gap-1">
        <div className="flex justify-between text-sm">
          <span>{t('page.usedToday')}</span>
          <span className="tabular-nums">
            {t('page.usage', { used: status.used_today, limit: status.daily_limit })}
          </span>
        </div>
        <div
          className="h-2 overflow-hidden rounded-full bg-muted"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={status.daily_limit}
          aria-valuenow={status.used_today}
          aria-label={t('page.usedToday')}
        >
          <div className="h-full bg-primary transition-all" style={{ width: `${percent}%` }} />
        </div>
        <p className="text-xs text-muted-foreground">{t('page.userLimit', { count: status.user_hourly_limit })}</p>
      </div>
    </div>
  )
}

function CallLog() {
  const t = useModuleTranslation(AI_KEY)
  const formatting = useRecordFormatting()
  const users = useOrganizationUsers()
  const [filters, setFilters] = useState<AiCallFilters>({ feature: null, status: null, page: 1 })
  const calls = useAiCalls(filters)
  const userName = (id: string | null) =>
    (id && users.data?.find((user) => user.id === id)?.display_name) || t('log.unknownUser')
  const pages = calls.data ? Math.max(1, Math.ceil(calls.data.meta.total / CALLS_PER_PAGE)) : 1

  return (
    <section className="grid gap-3" aria-label={t('log.title')}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-0.5">
          <h2 className="text-base font-semibold">{t('log.title')}</h2>
          <p className="text-sm text-muted-foreground">{t('log.description')}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <NativeSelect value={filters.feature ?? ALL}
            onValueChange={(value) =>
              setFilters((current) => ({ ...current, page: 1, feature: value === ALL ? null : (value as AiFeature) }))
            } className="w-44" aria-label={t('log.feature')}>
              <option value={ALL}>{t('log.allFeatures')}</option>
              {AI_FEATURES.map((feature) => (
                <option key={feature} value={feature}>
                  {t(`features.${feature}`)}
                </option>
              ))}
            </NativeSelect>
          <NativeSelect value={filters.status ?? ALL}
            onValueChange={(value) =>
              setFilters((current) => ({
                ...current,
                page: 1,
                status: value === ALL ? null : (value as 'success' | 'error'),
              }))
            } className="w-36" aria-label={t('log.status')}>
              <option value={ALL}>{t('log.allStatuses')}</option>
              <option value="success">{t('log.statuses.success')}</option>
              <option value="error">{t('log.statuses.error')}</option>
            </NativeSelect>
        </div>
      </div>
      {calls.data && (
        <dl className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {(
            [
              ['calls', calls.data.meta.last_30_days.calls],
              ['errors', calls.data.meta.last_30_days.errors],
              ['inputTokens', calls.data.meta.last_30_days.input_tokens],
              ['outputTokens', calls.data.meta.last_30_days.output_tokens],
            ] as const
          ).map(([key, value]) => (
            <div key={key} className="rounded-md border bg-card px-3 py-2">
              <dt className="text-xs text-muted-foreground">{t(`log.totals.${key}`)}</dt>
              <dd className="text-lg font-semibold tabular-nums">{value.toLocaleString(activeLocale())}</dd>
            </div>
          ))}
        </dl>
      )}
      {calls.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : calls.isError ? (
        <ErrorState onRetry={() => void calls.refetch()} />
      ) : calls.data.data.length === 0 ? (
        <EmptyState icon={Sparkles} title={t('log.emptyTitle')} description={t('log.emptyDescription')} />
      ) : (
        <ul className="grid gap-2">
          {calls.data.data.map((call) => (
            <li key={call.id} className="grid gap-1 rounded-md border bg-card px-3 py-2 text-sm sm:grid-cols-[1fr_auto] sm:items-center">
              <div className="grid min-w-0 gap-0.5">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium">{t(`features.${call.feature}`)}</span>
                  <Badge variant={call.status === 'success' ? 'secondary' : 'destructive'}>
                    {t(`log.statuses.${call.status}`)}
                  </Badge>
                  {call.error_code && <code className="text-xs text-muted-foreground">{call.error_code}</code>}
                </div>
                <p className="truncate text-xs text-muted-foreground">
                  {userName(call.user_id)} · {formatDateTime(call.created_at, formatting)} · {call.model}
                </p>
              </div>
              <p className="text-xs text-muted-foreground tabular-nums sm:text-right">
                {t('log.tokens', { input: call.input_tokens ?? '—', output: call.output_tokens ?? '—' })}
                {' · '}
                {t('log.duration', { seconds: (call.duration_ms / 1000).toFixed(1) })}
              </p>
            </li>
          ))}
        </ul>
      )}
      {calls.data && pages > 1 && (
        <div className="flex items-center justify-end gap-2">
          <Button
            size="sm"
            variant="outline"
            disabled={filters.page <= 1}
            onClick={() => setFilters((current) => ({ ...current, page: current.page - 1 }))}
            aria-label={t('log.previous')}
          >
            <ChevronLeft aria-hidden="true" />
          </Button>
          <span className="text-sm tabular-nums">{t('log.page', { page: filters.page, pages })}</span>
          <Button
            size="sm"
            variant="outline"
            disabled={filters.page >= pages}
            onClick={() => setFilters((current) => ({ ...current, page: current.page + 1 }))}
            aria-label={t('log.next')}
          >
            <ChevronRight aria-hidden="true" />
          </Button>
        </div>
      )}
    </section>
  )
}
