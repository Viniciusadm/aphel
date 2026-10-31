import { Plug } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useIntegrations, useInvalidateIntegrations } from '@/features/settings/email/integration/api'
import { IntegrationCard } from '@/features/settings/email/integration/IntegrationCard'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function IntegrationsPage() {
  const { t } = useTranslation('integrations')
  const integrations = useIntegrations()
  const invalidate = useInvalidateIntegrations()

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      {integrations.isPending ? (
        <div className="grid gap-3 md:grid-cols-2" aria-busy="true">
          <Skeleton className="h-44 w-full" />
          <Skeleton className="h-44 w-full" />
        </div>
      ) : integrations.isError ? (
        <ErrorState onRetry={() => void integrations.refetch()} />
      ) : integrations.data.length === 0 ? (
        <EmptyState icon={Plug} title={t('title')} description={t('description')} />
      ) : (
        <ul className="grid gap-3 md:grid-cols-2" aria-label={t('listLabel')}>
          {integrations.data.map((integration) => (
            <IntegrationCard key={integration.provider_key} integration={integration} onChanged={invalidate} />
          ))}
        </ul>
      )}
    </Page>
  )
}
