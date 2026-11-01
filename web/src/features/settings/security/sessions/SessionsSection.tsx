import { MonitorSmartphone } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { securitySessionsApi, useInvalidateSecuritySessions, useSecuritySessions, type SecuritySession } from './api'
import { DeviceIcon } from './DeviceIcon'
import { useDeviceLabel } from './device-label'
import { PageHeader } from '@/components/ui/page-header'
import { PageSection } from '@/components/ui/page'

export function SessionsSection() {
  const { t } = useTranslation('security')
  const sessions = useSecuritySessions()
  const invalidate = useInvalidateSecuritySessions()
  const deviceLabel = useDeviceLabel()
  const [revoking, setRevoking] = useState<SecuritySession | null>(null)
  const [revokingOthers, setRevokingOthers] = useState(false)
  const otherCount = sessions.data?.filter((session) => !session.current).length ?? 0

  const revoke = async (session: SecuritySession) => {
    await securitySessionsApi.revoke(session.id)
    await invalidate()
    notifySuccess(t('sessions.revoked'))
  }

  const revokeOthers = async () => {
    const result = await securitySessionsApi.revokeOthers()
    await invalidate()
    notifySuccess(t('sessions.revokedOthers', { count: result.revoked }))
  }

  return (
    <PageSection>
      <PageHeader level="section"
        title={t('sessions.title')}
        description={t('sessions.description')}
        actions={
          otherCount > 0 ? (
            <Button size="sm" variant="outline" onClick={() => setRevokingOthers(true)}>
              {t('sessions.revokeOthers')}
            </Button>
          ) : undefined
        }
      />
      {sessions.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : sessions.isError ? (
        <ErrorState onRetry={() => void sessions.refetch()} />
      ) : sessions.data.length === 0 ? (
        <EmptyState icon={MonitorSmartphone} title={t('sessions.empty')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('sessions.listLabel')}>
          {sessions.data.map((session) => (
            <SessionRow
              key={session.id}
              session={session}
              label={deviceLabel(session.device)}
              onRevoke={() => setRevoking(session)}
            />
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={revoking !== null}
        onOpenChange={(open) => !open && setRevoking(null)}
        title={t('sessions.revokeTitle')}
        description={t('sessions.revokeDescription', {
          device: revoking ? deviceLabel(revoking.device) : '',
        })}
        confirmLabel={t('sessions.revokeConfirm')}
        variant="destructive"
        onConfirm={() => (revoking ? revoke(revoking) : undefined)}
      />
      <ConfirmDialog
        open={revokingOthers}
        onOpenChange={setRevokingOthers}
        title={t('sessions.revokeOthersTitle')}
        description={t('sessions.revokeOthersDescription', { count: otherCount })}
        confirmLabel={t('sessions.revokeOthersConfirm')}
        variant="destructive"
        onConfirm={revokeOthers}
      />
    </PageSection>
  )
}

interface SessionRowProps {
  session: SecuritySession
  label: string
  onRevoke: () => void
}

function SessionRow({ session, label, onRevoke }: SessionRowProps) {
  const { t } = useTranslation('security')
  const formatting = useRecordFormatting()

  return (
    <li className="flex flex-col gap-2 rounded-md border bg-card px-3 py-2 sm:flex-row sm:items-center sm:justify-between">
      <div className="flex min-w-0 items-start gap-3">
        <DeviceIcon device={session.device} className="mt-0.5 size-5 shrink-0 text-muted-foreground" />
        <div className="grid min-w-0 gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium break-words">{label}</span>
            {session.current && <Badge variant="secondary">{t('sessions.current')}</Badge>}
          </div>
          <p className="text-xs break-words text-muted-foreground">
            {t('sessions.ip', { ip: session.ip ?? t('unknownIp') })}
            {' · '}
            {t('sessions.lastSeen', { date: formatDateTime(session.last_seen_at, formatting) })}
            {' · '}
            {t('sessions.createdAt', { date: formatDateTime(session.created_at, formatting) })}
          </p>
        </div>
      </div>
      {!session.current && (
        <Button
          size="sm"
          variant="outline"
          className="self-start sm:self-center"
          aria-label={t('sessions.revokeLabel', { device: label })}
          onClick={onRevoke}
        >
          {t('sessions.revoke')}
        </Button>
      )}
    </li>
  )
}
