import { KeyRound } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ConnectionCard, useProviderStatus } from '@/sdk'
import { SSO_KEY } from './api'

export function SsoCard() {
  const { t } = useTranslation('sso')
  const login = useProviderStatus(SSO_KEY).data?.login
  const state = login?.configured ? 'on' : 'off'
  return (
    <ConnectionCard
      icon={KeyRound}
      title={t('nav')}
      description={t('card.description')}
      state={state}
      stateLabel={t(`card.state.${state}`)}
      detail={
        !login?.configured
          ? t('card.missing')
          : login.require_sso
            ? t('card.required')
            : login.domains.length > 0
              ? t('card.domains', { domains: login.domains.join(', ') })
              : t('card.optional')
      }
      to="/integrations/sso"
      actionLabel={t('card.action')}
    />
  )
}
