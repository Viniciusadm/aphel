import { useQueryClient } from '@tanstack/react-query'
import { RefreshCw, ShieldAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { SYSTEM_MANAGE } from '@/features/settings/lib/permissions'
import { systemKeys, useSystemOverview } from './api'
import { FailedJobsSection } from './FailedJobsSection'
import { FailuresSection } from './FailuresSection'
import { ModulesSection, MigrationsSection } from './MigrationsSection'
import { QueueSection } from './QueueSection'
import { StatusSection } from './StatusSection'

const TWO_FACTOR_SETUP_REQUIRED = 'two_factor_setup_required'

export function SystemPage() {
  const { t } = useTranslation('system')
  const client = useQueryClient()
  const { hasPermission } = useSession()
  const allowed = hasPermission(SYSTEM_MANAGE)
  const overview = useSystemOverview(allowed)

  let body
  if (!allowed) {
    body = <EmptyState icon={ShieldAlert} title={t('forbidden.title')} description={t('forbidden.description')} />
  } else if (overview.isPending) {
    body = (
      <div className="grid gap-4" aria-busy="true">
        <Skeleton className="h-64 w-full" />
        <Skeleton className="h-40 w-full" />
      </div>
    )
  } else if (overview.isError) {
    const error = overview.error
    if (isApiError(error) && error.status === 403) {
      body = (
        <EmptyState
          icon={ShieldAlert}
          title={t('forbidden.title')}
          description={
            error.code === TWO_FACTOR_SETUP_REQUIRED ? t('forbidden.twoFactor') : t('forbidden.description')
          }
        />
      )
    } else {
      body = <ErrorState onRetry={() => void overview.refetch()} />
    }
  } else {
    body = (
      <div className="grid gap-4">
        <StatusSection overview={overview.data} />
        <QueueSection />
        <FailedJobsSection />
        <FailuresSection />
        <div className="grid gap-4 xl:grid-cols-2">
          <MigrationsSection />
          <ModulesSection modules={overview.data.modules} />
        </div>
      </div>
    )
  }

  return (
    <div className="flex min-w-0 flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-1">
          <h1 className="text-xl font-semibold">{t('page.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('page.description')}</p>
        </div>
        {overview.isSuccess && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => void client.invalidateQueries({ queryKey: systemKeys.all })}
          >
            <RefreshCw aria-hidden="true" />
            {t('page.refresh')}
          </Button>
        )}
      </div>
      {body}
    </div>
  )
}
