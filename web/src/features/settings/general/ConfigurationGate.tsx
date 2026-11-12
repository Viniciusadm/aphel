import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { PageLoader } from '@/components/feedback'
import { ErrorState } from '@/components/ui/error-state'
import { useOrganizationConfiguration, type OrganizationConfiguration } from './api'

interface ConfigurationGateProps {
  children: (configuration: OrganizationConfiguration) => ReactNode
  fallback?: (content: ReactNode) => ReactNode
}

export function ConfigurationGate({ children, fallback = (content) => content }: ConfigurationGateProps) {
  const { t } = useTranslation('settings')
  const query = useOrganizationConfiguration()
  if (query.isPending) return <>{fallback(<PageLoader />)}</>
  if (query.isError) {
    return <>{fallback(<ErrorState title={t('configuration.loadError')} onRetry={() => void query.refetch()} />)}</>
  }
  return <>{children(query.data)}</>
}
