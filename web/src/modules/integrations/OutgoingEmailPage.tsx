import { useTranslation } from 'react-i18next'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { MailNotConfiguredNotice } from '@/features/settings/email/MailNotConfiguredNotice'
import { SmtpIntegrationPanel } from '@/features/settings/email/SmtpIntegrationPanel'

export function OutgoingEmailPage() {
  const { t } = useTranslation('settings')
  return (
    <section className="grid gap-6">
      <SectionHeader title={t('configuration.mail.title')} description={t('configuration.mail.description')} />
      <MailNotConfiguredNotice showLink={false} />
      <SmtpIntegrationPanel />
      <p className="max-w-2xl text-sm text-muted-foreground">{t('configuration.mail.testHint')}</p>
    </section>
  )
}
