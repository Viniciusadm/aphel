import { Sparkles } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ConnectionCard } from '@/sdk'
import { useAiStatus } from './api'

export function AiCard() {
  const { t } = useTranslation('ai')
  const status = useAiStatus().data
  const state = status?.configured ? 'on' : 'off'
  return (
    <ConnectionCard
      icon={Sparkles}
      title={t('nav')}
      description={t('card.description')}
      state={state}
      stateLabel={t(`card.state.${state}`)}
      detail={status?.configured ? t('card.model', { model: status.model }) : t('card.missing')}
      to="/integrations/ai"
      actionLabel={t('card.action')}
    />
  )
}
