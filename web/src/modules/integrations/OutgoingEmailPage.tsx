import { useTranslation } from 'react-i18next'
import { MailNotConfiguredNotice } from '@/features/settings/email/MailNotConfiguredNotice'
import { SmtpIntegrationPanel } from '@/features/settings/email/SmtpIntegrationPanel'
import { PageHeader } from '@/components/ui/page-header'

export function OutgoingEmailPage() {
  const { t } = useTranslation('settings')
  return (
    <section className="grid gap-6">
      <PageHeader title={t('configuration.mail.title')} description={t('configuration.mail.description')} />
      <MailNotConfiguredNotice showLink={false} />
      <SmtpIntegrationPanel />
      <p className="max-w-2xl text-sm text-muted-foreground">{t('configuration.mail.testHint')}</p>
    </section>
  )
}
