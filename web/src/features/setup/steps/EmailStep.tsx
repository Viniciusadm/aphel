import { useTranslation } from 'react-i18next'
import { SMTP_PROVIDER_KEY, SmtpIntegrationPanel } from '@/features/settings/email/SmtpIntegrationPanel'
import { useIntegrations } from '@/features/settings/email/integration/api'
import { SetupStepFrame } from '@/kernel/setup'

export function EmailStep({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation('setup')
  const integrations = useIntegrations()
  const configured = integrations.data?.some(
    (integration) => integration.provider_key === SMTP_PROVIDER_KEY && integration.configured,
  )

  return (
    <SetupStepFrame
      title={t('email.title')}
      description={t('email.description')}
      primary={configured ? { label: t('email.continue'), onClick: onDone } : undefined}
      secondary={configured ? undefined : { label: t('actions.later'), onClick: onDone }}
    >
      <SmtpIntegrationPanel />
      {!configured && <p className="text-sm text-muted-foreground">{t('email.later')}</p>}
    </SetupStepFrame>
  )
}
