import { Fragment } from 'react'
import { Plug } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { useSlot } from '@/kernel/modules/slots'
import { IntegrationCard } from './providers/IntegrationCard'
import { useIntegrations, useInvalidateIntegrations } from './providers/api'

export function IntegrationsPage() {
  const { t } = useTranslation('integrations')
  const cards = useSlot('integrations.cards', {})
  const providers = useIntegrations()
  const invalidate = useInvalidateIntegrations()
  const configurable = providers.data ?? []

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      {cards.length === 0 && configurable.length === 0 && !providers.isPending ? (
        providers.isError ? (
          <ErrorState onRetry={() => void providers.refetch()} />
        ) : (
          <EmptyState icon={Plug} title={t('empty.title')} description={t('empty.description')} />
        )
      ) : (
        <ul className="grid gap-3 md:grid-cols-2" aria-label={t('listLabel')}>
          {cards.map(({ key, Component }) => (
            <Fragment key={key}>
              <Component />
            </Fragment>
          ))}
          {configurable.map((integration) => (
            <IntegrationCard key={integration.provider_key} integration={integration} onChanged={invalidate} />
          ))}
        </ul>
      )}
    </Page>
  )
}
