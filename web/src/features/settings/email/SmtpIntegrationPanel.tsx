import { useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Skeleton } from '@/components/ui/skeleton'
import { ErrorState } from '@/components/ui/error-state'
import { useIntegrations, useInvalidateIntegrations } from './integration/api'
import { IntegrationCard } from './integration/IntegrationCard'
import { configurationKeys } from '../general/api'

export const SMTP_PROVIDER_KEY = 'smtp'

export function SmtpIntegrationPanel() {
  const { t } = useTranslation('settings')
  const integrations = useIntegrations()
  const invalidateIntegrations = useInvalidateIntegrations()
  const queryClient = useQueryClient()
  const onChanged = () =>
    Promise.all([invalidateIntegrations(), queryClient.invalidateQueries({ queryKey: configurationKeys.all })])

  if (integrations.isPending) return <Skeleton className="h-44 w-full max-w-xl" />
  if (integrations.isError) return <ErrorState onRetry={() => void integrations.refetch()} />
  const smtp = integrations.data.find((integration) => integration.provider_key === SMTP_PROVIDER_KEY)
  if (!smtp) return <p className="text-sm text-muted-foreground">{t('configuration.mail.unavailable')}</p>
  return (
    <ul className="grid max-w-xl gap-3" aria-label={t('configuration.mail.title')}>
      <IntegrationCard integration={smtp} onChanged={onChanged} />
    </ul>
  )
}
