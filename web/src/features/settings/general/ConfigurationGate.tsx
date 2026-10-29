import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { PageLoader } from '@/components/feedback'
import { ErrorState } from '@/components/ui/error-state'
import { useOrganizationConfiguration, type OrganizationConfiguration } from './api'

interface ConfigurationGateProps {
  children: (configuration: OrganizationConfiguration) => ReactNode
}

export function ConfigurationGate({ children }: ConfigurationGateProps) {
  const { t } = useTranslation('settings')
  const query = useOrganizationConfiguration()
  if (query.isPending) return <PageLoader />
  if (query.isError) return <ErrorState title={t('configuration.loadError')} onRetry={() => void query.refetch()} />
  return <>{children(query.data)}</>
}
