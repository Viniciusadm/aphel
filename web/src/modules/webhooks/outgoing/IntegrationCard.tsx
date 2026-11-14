import { Send } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ConnectionCard } from '@/sdk'
import { useWebhookEndpoints } from './api'

export function OutgoingWebhooksCard() {
  const { t } = useTranslation('webhooks')
  const endpoints = useWebhookEndpoints().data ?? []
  const active = endpoints.filter((endpoint) => endpoint.is_active)
  const problems = endpoints.filter((endpoint) => !endpoint.is_active || endpoint.consecutive_failures > 0)
  const state = problems.length > 0 ? 'attention' : active.length > 0 ? 'on' : 'off'
  return (
    <ConnectionCard
      icon={Send}
      title={t('title')}
      description={t('card.description')}
      state={state}
      stateLabel={t(`card.state.${state}`)}
      detail={
        endpoints.length === 0
          ? t('card.none')
          : problems.length > 0
            ? t('card.problems', { count: problems.length })
            : t('card.active', { count: active.length })
      }
      to="/integrations/webhooks"
      actionLabel={t('card.action')}
    />
  )
}
