import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { useDateTimeFormat, useDurationFormat } from './format'
import { SectionCard } from './SectionCard'
import { asCheckStatus, type CheckStatus, type HealthCheck, type SystemOverview } from './types'

const CHECK_NAMES = ['database', 'storage', 'worker', 'scheduler', 'queue'] as const
const CHECK_REASONS = [
  'unreachable',
  'unavailable',
  'database_unreachable',
  'query_failed',
  'no_heartbeat',
  'heartbeat_stale',
  'never_started',
  'schedules_overdue',
  'backlog_delayed',
  'disabled',
] as const

type CheckName = (typeof CHECK_NAMES)[number]
type CheckReason = (typeof CHECK_REASONS)[number]

function isCheckName(value: string): value is CheckName {
  return (CHECK_NAMES as readonly string[]).includes(value)
}

function isCheckReason(value: string): value is CheckReason {
  return (CHECK_REASONS as readonly string[]).includes(value)
}

const STATUS_VARIANT: Record<CheckStatus, 'success' | 'destructive' | 'secondary'> = {
  ok: 'success',
  fail: 'destructive',
  skipped: 'secondary',
}

function CheckRow({ check }: { check: HealthCheck }) {
  const { t } = useTranslation('system')
  const duration = useDurationFormat()
  const status = asCheckStatus(check.status)
  const name = isCheckName(check.name) ? t(`checks.names.${check.name}`) : check.name
  const reason = check.reason ? (isCheckReason(check.reason) ? t(`checks.reasons.${check.reason}`) : check.reason) : null
  const age =
    typeof check.age_seconds === 'number' && typeof check.threshold_seconds === 'number'
      ? t('checks.age', { age: duration(check.age_seconds), limit: duration(check.threshold_seconds) })
      : null

  return (
    <li className="flex flex-col gap-1 py-2 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
      <div className="grid min-w-0 gap-0.5">
        <span className="font-medium">{name}</span>
        {reason && <span className="text-sm text-muted-foreground">{reason}</span>}
        {age && <span className="text-xs text-muted-foreground">{age}</span>}
      </div>
      <Badge variant={STATUS_VARIANT[status]}>{t(`checks.status.${status}`)}</Badge>
    </li>
  )
}

export function StatusSection({ overview }: { overview: SystemOverview }) {
  const { t } = useTranslation('system')
  const format = useDateTimeFormat()
  const ready = overview.readiness.status === 'ok'

  return (
    <SectionCard
      title={t('status.title')}
      description={t('status.checkedAt', { date: format.format(new Date(overview.readiness.checked_at)) })}
      action={<Badge variant={ready ? 'success' : 'destructive'}>{t(ready ? 'status.ready' : 'status.notReady')}</Badge>}
    >
      <dl className="grid gap-3 text-sm sm:grid-cols-3">
        <div className="grid gap-0.5">
          <dt className="text-muted-foreground">{t('status.version')}</dt>
          <dd className="font-mono">{overview.version}</dd>
        </div>
        <div className="grid gap-0.5">
          <dt className="text-muted-foreground">{t('status.platform')}</dt>
          <dd className="font-mono">
            {overview.os} / {overview.arch}
          </dd>
        </div>
      </dl>
      <ul aria-label={t('status.checksLabel')} className="divide-y rounded-lg border px-4">
        {overview.readiness.checks.map((check) => (
          <CheckRow key={check.name} check={check} />
        ))}
      </ul>
      <div className="grid gap-2">
        <h4 className="text-sm font-medium">{t('workers.title')}</h4>
        {overview.heartbeats.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('workers.empty')}</p>
        ) : (
          <ul className="grid gap-2 text-sm">
            {overview.heartbeats.map((heartbeat) => (
              <li key={heartbeat.instance_id} className="grid gap-0.5 rounded-md border px-3 py-2">
                <span className="font-mono text-xs break-all">
                  {heartbeat.instance_id}
                  {heartbeat.hostname ? ` @ ${heartbeat.hostname}` : ''} · {heartbeat.version}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t('workers.lastSeen', { date: format.format(new Date(heartbeat.last_seen_at)) })} ·{' '}
                  {t('workers.startedAt', { date: format.format(new Date(heartbeat.started_at)) })}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </SectionCard>
  )
}
