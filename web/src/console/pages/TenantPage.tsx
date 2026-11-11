import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Pause, Play, Trash2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useNavigate, useParams } from 'react-router'
import { isApiError } from '@/api'
import { Alert } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { PageLoader } from '@/components/ui/page-loader'
import { ToggleField } from '@/components/ui/toggle-field'
import { consoleApi, consoleKeys, type Tenant } from '../api'
import { TenantHealthCard } from '../components/TenantHealthCard'
import { Notice } from '../components/Notice'
import { TenantHostsCard } from '../components/TenantHostsCard'
import { TenantStatusBadge } from '../components/TenantStatusBadge'
import { notifyConsoleError, notifyConsoleSuccess } from '../errors'
import { formatBytes, formatDateTime } from '../format'
import { TENANTS_PATH } from '../paths'
import { isTransitional } from '../status'

const POLL_INTERVAL = 3_000

export function TenantPage() {
  const { t } = useTranslation('console')
  const { slug = '' } = useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [confirming, setConfirming] = useState<'suspend' | 'delete' | null>(null)
  const [exportCopy, setExportCopy] = useState(true)
  const tenant = useQuery({
    queryKey: consoleKeys.tenant(slug),
    queryFn: () => consoleApi.tenant(slug),
    retry: false,
    refetchInterval: (query) => (query.state.data && isTransitional(query.state.data.status) ? POLL_INTERVAL : false),
  })
  const catalog = useQuery({ queryKey: consoleKeys.catalog, queryFn: consoleApi.catalog })
  const exportAvailable = catalog.data?.export_available ?? false

  const store = (updated: Tenant) => {
    queryClient.setQueryData(consoleKeys.tenant(slug), updated)
    void queryClient.invalidateQueries({ queryKey: consoleKeys.tenants, exact: true })
    void queryClient.invalidateQueries({ queryKey: consoleKeys.health(slug) })
  }
  const suspend = useMutation({
    mutationFn: () => consoleApi.suspend(slug),
    onSuccess: (updated) => {
      store(updated)
      notifyConsoleSuccess(t('tenant.suspended'))
    },
  })
  const resume = useMutation({
    mutationFn: () => consoleApi.resume(slug),
    onSuccess: (updated) => {
      store(updated)
      notifyConsoleSuccess(t('tenant.resumed'))
    },
    onError: notifyConsoleError,
  })
  const remove = useMutation({
    mutationFn: () => consoleApi.deleteTenant(slug, exportAvailable && exportCopy),
    onSuccess: (updated) => {
      store(updated)
      notifyConsoleSuccess(t('tenant.deleteStarted'))
    },
  })

  const gone = tenant.isError && isApiError(tenant.error) && tenant.error.status === 404
  useEffect(() => {
    if (gone) void queryClient.invalidateQueries({ queryKey: consoleKeys.tenants, exact: true })
  }, [gone, queryClient])

  if (tenant.isPending) return <PageLoader />
  if (gone) return <Navigate to={TENANTS_PATH} replace />
  if (!tenant.data) return <ErrorState onRetry={() => void tenant.refetch()} />

  const data = tenant.data
  const active = data.status === 'active'
  const busy = suspend.isPending || resume.isPending || remove.isPending
  const actions = (
    <div className="flex flex-wrap gap-2">
      {active && (
        <Button variant="outline" disabled={busy} onClick={() => setConfirming('suspend')}>
          <Pause className="size-4" aria-hidden />
          {t('tenant.suspend')}
        </Button>
      )}
      {data.status === 'suspended' && (
        <Button variant="outline" disabled={busy} onClick={() => resume.mutate()}>
          <Play className="size-4" aria-hidden />
          {t('tenant.resume')}
        </Button>
      )}
      {(data.status !== 'deleting' || data.last_error) && (
        <Button variant="destructive" disabled={busy} onClick={() => setConfirming('delete')}>
          <Trash2 className="size-4" aria-hidden />
          {t('tenant.delete')}
        </Button>
      )}
    </div>
  )

  return (
    <Page>
      <PageHeader
        title={data.name}
        description={data.slug}
        badges={<TenantStatusBadge status={data.status} />}
        back={{ to: TENANTS_PATH, label: t('tenant.back') }}
        actions={actions}
      />
      <div className="grid gap-4">
        {data.status === 'provisioning' && !data.last_error && <Notice>{t('tenant.provisioning')}</Notice>}
        {data.status === 'deleting' && !data.last_error && <Notice>{t('tenant.deleting')}</Notice>}
        {data.status === 'suspended' && <Notice>{t('tenant.suspendedNotice')}</Notice>}
        {data.last_error && (
          <Alert role="alert" className="border-destructive/30 bg-destructive/10 text-sm text-destructive">
            <strong>{t('tenant.lastError')}:</strong> {data.last_error}
          </Alert>
        )}
        <div className="grid gap-4 lg:grid-cols-2">
          <TenantHostsCard tenant={data} editable={data.status !== 'deleting'} />
          <Card>
            <CardHeader>
              <CardTitle>{t('tenant.database')}</CardTitle>
            </CardHeader>
            <CardContent>
              <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
                <dt className="text-muted-foreground">{t('tenant.databaseName')}</dt>
                <dd className="font-mono">{data.database_name}</dd>
                <dt className="text-muted-foreground">{t('tenant.size')}</dt>
                <dd>{formatBytes(data.database_bytes) ?? t('tenants.unknown')}</dd>
                <dt className="text-muted-foreground">{t('tenant.lastWrite')}</dt>
                <dd>{formatDateTime(data.last_write_at) ?? t('tenants.unknown')}</dd>
                <dt className="text-muted-foreground">{t('tenant.migratedAt')}</dt>
                <dd>{formatDateTime(data.migrated_at) ?? t('tenant.never')}</dd>
                <dt className="text-muted-foreground">{t('tenant.createdAt')}</dt>
                <dd>{formatDateTime(data.created_at)}</dd>
              </dl>
            </CardContent>
          </Card>
        </div>
        <TenantHealthCard slug={data.slug} active={active} />
      </div>
      <ConfirmDialog
        open={confirming === 'suspend'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={t('tenant.suspendTitle', { name: data.name })}
        description={t('tenant.suspendDescription')}
        confirmLabel={t('tenant.suspend')}
        pending={suspend.isPending}
        onError={notifyConsoleError}
        onConfirm={async () => {
          await suspend.mutateAsync()
          setConfirming(null)
        }}
      />
      <ConfirmDialog
        open={confirming === 'delete'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={t('tenant.deleteTitle', { name: data.name })}
        description={t('tenant.deleteDescription')}
        confirmLabel={t('tenant.delete')}
        variant="destructive"
        confirmationText={data.slug}
        pending={remove.isPending}
        onError={notifyConsoleError}
        onConfirm={async () => {
          await remove.mutateAsync()
          setConfirming(null)
          void navigate(TENANTS_PATH)
        }}
      >
        {exportAvailable && (
          <ToggleField
            label={t('tenant.deleteExport')}
            description={t('tenant.deleteExportHelp')}
            checked={exportCopy}
            onCheckedChange={setExportCopy}
          />
        )}
      </ConfirmDialog>
    </Page>
  )
}
