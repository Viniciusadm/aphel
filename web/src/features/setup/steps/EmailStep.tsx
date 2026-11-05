import { useTranslation } from 'react-i18next'
import { MailSettingsForm } from '@/features/settings/email/OutgoingEmailPage'
import { ConfigurationGate } from '@/features/settings/general/ConfigurationGate'
import { useOrganizationConfiguration } from '@/features/settings/general/api'
import { SetupStepFrame } from '@/kernel/setup'

export function EmailStep({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation('setup')
  const configuration = useOrganizationConfiguration()
  const configured = configuration.data?.mail_configured

  return (
    <SetupStepFrame
      title={t('email.title')}
      description={t('email.description')}
      form={false}
      primary={configured ? { label: t('email.continue'), onClick: onDone } : undefined}
      secondary={configured ? undefined : { label: t('actions.later'), onClick: onDone }}
    >
      <ConfigurationGate>{(settings) => <MailSettingsForm configuration={settings} />}</ConfigurationGate>
      {!configured && <p className="text-sm text-muted-foreground">{t('email.later')}</p>}
    </SetupStepFrame>
  )
}
