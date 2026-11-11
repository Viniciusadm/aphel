import { useQuery } from '@tanstack/react-query'
import { AlertTriangle, Building2, Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { QueryState } from '@/components/ui/query-state'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { consoleApi, consoleKeys, type Tenant } from '../api'
import { TenantStatusBadge } from '../components/TenantStatusBadge'
import { formatBytes, formatDateTime } from '../format'
import { NEW_TENANT_PATH, tenantPath } from '../paths'
import { isTransitional } from '../status'

const POLL_INTERVAL = 4_000

export function TenantsPage() {
  const { t } = useTranslation('console')
  const navigate = useNavigate()
  const tenants = useQuery({
    queryKey: consoleKeys.tenants,
    queryFn: consoleApi.tenants,
    refetchInterval: (query) => (query.state.data?.some((tenant) => isTransitional(tenant.status)) ? POLL_INTERVAL : false),
  })

  const newTenant = (
    <Button asChild>
      <Link to={NEW_TENANT_PATH}>
        <Plus className="size-4" aria-hidden />
        {t('tenants.new')}
      </Link>
    </Button>
  )

  return (
    <Page>
      <PageHeader title={t('tenants.title')} description={t('tenants.description')} actions={newTenant} />
      <QueryState
        query={tenants}
        isEmpty={(data: Tenant[]) => data.length === 0}
        empty={{ icon: Building2, title: t('tenants.empty'), description: t('tenants.emptyDescription'), action: newTenant }}
      >
        {(data: Tenant[]) => (
          <Card className="overflow-x-auto p-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t('tenants.columns.company')}</TableHead>
                  <TableHead>{t('tenants.columns.status')}</TableHead>
                  <TableHead>{t('tenants.columns.address')}</TableHead>
                  <TableHead className="text-right">{t('tenants.columns.size')}</TableHead>
                  <TableHead>{t('tenants.columns.lastWrite')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.map((tenant) => {
                  const primary = tenant.hosts.find((host) => host.is_primary) ?? tenant.hosts[0]
                  return (
                    <TableRow key={tenant.id} className="cursor-pointer" onClick={() => void navigate(tenantPath(tenant.slug))}>
                      <TableCell>
                        <Link to={tenantPath(tenant.slug)} className="font-medium hover:underline" onClick={(event) => event.stopPropagation()}>
                          {tenant.name}
                        </Link>
                        <div className="text-xs text-muted-foreground">{tenant.slug}</div>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <TenantStatusBadge status={tenant.status} />
                          {tenant.last_error && (
                            <AlertTriangle className="size-4 text-destructive" aria-label={t('tenants.problem')} />
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="text-sm">{primary?.host ?? t('tenants.noHost')}</TableCell>
                      <TableCell className="text-right text-sm tabular-nums">{formatBytes(tenant.database_bytes) ?? t('tenants.unknown')}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">{formatDateTime(tenant.last_write_at) ?? t('tenants.unknown')}</TableCell>
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          </Card>
        )}
      </QueryState>
    </Page>
  )
}
