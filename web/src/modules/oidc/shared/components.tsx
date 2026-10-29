import { CalendarDays, Check, Copy, KeyRound, RefreshCw, Unplug } from 'lucide-react'
import { useEffect, useState, type ReactNode } from 'react'
import { useSearchParams } from 'react-router'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {
  SSO_SHARED_KEY,
  useCalendarActions,
  useProviderStatus,
  type CalendarStatus,
  type SsoLoginStatus,
} from './api'

const KNOWN_CALENDAR_ERRORS = [
  'access_denied',
  'permission_denied',
  'calendar_no_refresh_token',
  'calendar_authorization_revoked',
  'calendar_not_connected',
  'calendar_member_unavailable',
  'calendar_permission_revoked',
  'calendar_meeting_type_not_found',
  'calendar_api_failed',
  'sso_not_configured',
  'sso_disabled',
  'invalid_state',
  'token_exchange_failed',
  'invalid_token',
  'provider_unavailable',
]

function calendarErrorKey(code: string | null | undefined): string {
  return code && KNOWN_CALENDAR_ERRORS.includes(code) ? `shared.calendarErrors.${code}` : 'shared.calendarErrors.generic'
}

export function CallbackUrlField({ url }: { url: string }) {
  const t = useModuleTranslation(SSO_SHARED_KEY)
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      notifyError(error)
    }
  }
  return (
    <div className="grid gap-1.5">
      <span className="text-sm font-medium">{t('shared.callbackUrl')}</span>
      <div className="flex gap-2">
        <Input readOnly value={url} className="min-w-0 font-mono text-xs" aria-label={t('shared.callbackUrl')} />
        <Button type="button" variant="outline" size="sm" onClick={() => void copy()}>
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          <span className="sr-only sm:not-sr-only">{copied ? t('shared.copied') : t('shared.copy')}</span>
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">{t('shared.callbackHelp')}</p>
    </div>
  )
}

export function SsoStatusSection({ status }: { status: SsoLoginStatus }) {
  const t = useModuleTranslation(SSO_SHARED_KEY)
  return (
    <Card className="gap-4">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <KeyRound className="size-4" aria-hidden="true" />
          {t('shared.login.title')}
        </CardTitle>
        <CardDescription>{t('shared.login.description', { label: status.label })}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <div className="flex flex-wrap gap-2">
          <Badge variant={status.configured ? 'success' : 'warning'}>
            {status.configured ? t('shared.login.configured') : t('shared.login.notConfigured')}
          </Badge>
          {status.require_sso && <Badge variant="secondary">{t('shared.login.required')}</Badge>}
        </div>
        <div className="grid gap-1 text-sm">
          <span className="font-medium">{t('shared.login.domains')}</span>
          {status.domains.length > 0 ? (
            <ul className="flex flex-wrap gap-1.5">
              {status.domains.map((domain) => (
                <li key={domain}>
                  <Badge variant="outline">@{domain}</Badge>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-muted-foreground">{t('shared.login.noDomains')}</p>
          )}
        </div>
        <CallbackUrlField url={status.callback_url} />
      </CardContent>
    </Card>
  )
}

function useCalendarRedirectFeedback() {
  const t = useModuleTranslation(SSO_SHARED_KEY)
  const [searchParams, setSearchParams] = useSearchParams()
  const connected = searchParams.get('calendar')
  const failed = searchParams.get('calendar_error')
  useEffect(() => {
    if (!connected && !failed) return
    if (connected === 'connected') notifySuccess(t('shared.calendar.connectedToast'))
    if (failed) toast({ title: t(calendarErrorKey(failed)), variant: 'destructive' })
    const next = new URLSearchParams(searchParams)
    next.delete('calendar')
    next.delete('calendar_error')
    setSearchParams(next, { replace: true })
  }, [connected, failed, searchParams, setSearchParams, t])
}

interface CalendarSectionProps {
  moduleKey: string
  providerName: string
  status: CalendarStatus
}

export function CalendarSection({ moduleKey, providerName, status }: CalendarSectionProps) {
  const t = useModuleTranslation(SSO_SHARED_KEY)
  const formatting = useRecordFormatting()
  const { connect, disconnect, sync } = useCalendarActions(moduleKey)
  const [confirming, setConfirming] = useState(false)
  useCalendarRedirectFeedback()
  const connection = status.connection

  const startConnection = () =>
    connect.mutate(undefined, {
      onError: (error) => notifyError(error),
    })
  const syncNow = () =>
    sync.mutate(undefined, {
      onSuccess: () => notifySuccess(t('shared.calendar.syncQueued')),
      onError: (error) => notifyError(error),
    })

  let body: ReactNode
  if (!status.configured) {
    body = <p className="text-sm text-muted-foreground">{t('shared.calendar.notConfigured')}</p>
  } else if (!connection) {
    body = (
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">{t('shared.calendar.disconnected', { provider: providerName })}</p>
        {status.can_connect ? (
          <Button size="sm" onClick={startConnection} disabled={connect.isPending}>
            <CalendarDays aria-hidden="true" />
            {t('shared.calendar.connect')}
          </Button>
        ) : (
          <p className="text-xs text-muted-foreground">{t('shared.calendar.noPermission')}</p>
        )}
      </div>
    )
  } else {
    const stateVariant = connection.status === 'active' ? 'success' : connection.status === 'revoked' ? 'destructive' : 'warning'
    body = (
      <div className="grid gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant={stateVariant}>{t(`shared.calendar.states.${connection.status}`, { defaultValue: connection.status })}</Badge>
          {connection.account_email && <span className="text-sm break-all">{connection.account_email}</span>}
        </div>
        <dl className="grid gap-1 text-sm sm:grid-cols-2">
          <div className="flex gap-1">
            <dt className="text-muted-foreground">{t('shared.calendar.lastSync')}</dt>
            <dd>{connection.last_synced_at ? formatDateTime(connection.last_synced_at, formatting) : t('shared.calendar.never')}</dd>
          </div>
          <div className="flex gap-1">
            <dt className="text-muted-foreground">{t('shared.calendar.counters')}</dt>
            <dd>
              {t('shared.calendar.countersValue', {
                imported: connection.imported_count,
                exported: connection.exported_count,
              })}
            </dd>
          </div>
        </dl>
        {connection.last_error_code && (
          <p role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm">
            {t(calendarErrorKey(connection.last_error_code))}
            {connection.last_error_message && (
              <span className="mt-1 block text-xs text-muted-foreground">{connection.last_error_message}</span>
            )}
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          {connection.status === 'revoked' && status.can_connect ? (
            <Button size="sm" onClick={startConnection} disabled={connect.isPending}>
              <CalendarDays aria-hidden="true" />
              {t('shared.calendar.reconnect')}
            </Button>
          ) : (
            <Button size="sm" variant="outline" onClick={syncNow} disabled={sync.isPending || !status.can_connect}>
              <RefreshCw aria-hidden="true" />
              {t('shared.calendar.syncNow')}
            </Button>
          )}
          <Button size="sm" variant="outline" onClick={() => setConfirming(true)} disabled={disconnect.isPending}>
            <Unplug aria-hidden="true" />
            {t('shared.calendar.disconnect')}
          </Button>
        </div>
      </div>
    )
  }

  return (
    <Card className="gap-4">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <CalendarDays className="size-4" aria-hidden="true" />
          {t('shared.calendar.title', { provider: providerName })}
        </CardTitle>
        <CardDescription>{t('shared.calendar.description', { provider: providerName })}</CardDescription>
      </CardHeader>
      <CardContent>{body}</CardContent>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('shared.calendar.confirmTitle', { provider: providerName })}
        description={t('shared.calendar.confirmDescription')}
        confirmLabel={t('shared.calendar.disconnect')}
        variant="destructive"
        onConfirm={async () => {
          await disconnect.mutateAsync()
          notifySuccess(t('shared.calendar.disconnectedToast'))
        }}
      />
    </Card>
  )
}

interface ProviderPageProps {
  moduleKey: string
  title: string
  description: string
  providerName: string
}

export function ProviderPage({ moduleKey, title, description, providerName }: ProviderPageProps) {
  const status = useProviderStatus(moduleKey)
  return (
    <section className="flex flex-col gap-4">
      <div className="grid gap-1">
        <h1 className="text-xl font-semibold">{title}</h1>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      {status.isPending ? (
        <div className="grid gap-3" aria-busy="true">
          <Skeleton className="h-40 w-full" />
          <Skeleton className="h-32 w-full" />
        </div>
      ) : status.isError ? (
        <ErrorState onRetry={() => void status.refetch()} />
      ) : (
        <div className="grid gap-4 lg:grid-cols-2 lg:items-start">
          {status.data.calendar && (
            <CalendarSection moduleKey={moduleKey} providerName={providerName} status={status.data.calendar} />
          )}
          <SsoStatusSection status={status.data.login} />
        </div>
      )}
    </section>
  )
}

interface ProviderSettingsPanelProps {
  moduleKey: string
  steps: string[]
}

export function ProviderSettingsPanel({ moduleKey, steps }: ProviderSettingsPanelProps) {
  const t = useModuleTranslation(SSO_SHARED_KEY)
  const status = useProviderStatus(moduleKey)
  return (
    <div className="grid gap-3 text-sm">
      {status.isPending ? (
        <Skeleton className="h-16 w-full" />
      ) : status.isError ? (
        <p className="text-muted-foreground">{t('shared.statusUnavailable')}</p>
      ) : (
        <CallbackUrlField url={status.data.login.callback_url} />
      )}
      <ol className="grid list-decimal gap-1 pl-5 text-muted-foreground">
        {steps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
    </div>
  )
}
