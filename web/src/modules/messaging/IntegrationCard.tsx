import { RadioTower } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ConnectionCard } from '@/sdk'
import { useChannels } from './api'

export function ChannelsCard() {
  const { t } = useTranslation('messaging')
  const channels = useChannels().data?.data ?? []
  const active = channels.filter((channel) => channel.is_active)
  const failing = active.filter((channel) => channel.status === 'error')
  const state = failing.length > 0 ? 'attention' : active.length > 0 ? 'on' : 'off'
  return (
    <ConnectionCard
      icon={RadioTower}
      title={t('nav.channels')}
      description={t('card.description')}
      state={state}
      stateLabel={t(`card.state.${state}`)}
      detail={
        channels.length === 0
          ? t('card.none')
          : failing.length > 0
            ? t('card.failing', { count: failing.length })
            : t('card.active', { count: active.length })
      }
      to="/integrations/channels"
      actionLabel={t('card.action')}
    />
  )
}
