import { Webhook } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ConnectionCard } from '@/sdk'
import { useIncomingWebhooks } from './api'

export function IncomingWebhooksCard() {
  const { t } = useTranslation('webhooks')
  const hooks = useIncomingWebhooks().data ?? []
  const active = hooks.filter((hook) => hook.is_active)
  const state = active.length > 0 ? 'on' : 'off'
  return (
    <ConnectionCard
      icon={Webhook}
      title={t('incoming.title')}
      description={t('incoming.card.description')}
      state={state}
      stateLabel={t(`card.state.${state}`)}
      detail={hooks.length === 0 ? t('incoming.card.none') : t('incoming.card.active', { count: active.length })}
      to="/integrations/incoming-webhooks"
      actionLabel={t('card.action')}
    />
  )
}
