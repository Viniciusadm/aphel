import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { SessionProvider } from '@/app/session/SessionProvider'
import { Button } from '@/components/ui/button'
import { AuthCard } from '@/features/auth/components/AuthCard'
import { SmtpIntegrationPanel } from '@/features/settings/email/SmtpIntegrationPanel'
import type { InstallerMe } from './api'

export function EmailStep({ me }: { me: InstallerMe }) {
  const { t } = useTranslation('installer')
  const navigate = useNavigate()
  const finish = () => void navigate('/', { replace: true })
  return (
    <AuthCard title={t('email.title')} description={t('email.description')}>
      <div className="grid gap-4">
        <SessionProvider me={me}>
          <SmtpIntegrationPanel />
        </SessionProvider>
        <p className="text-sm text-muted-foreground">{t('email.later')}</p>
        <div className="grid gap-2">
          <Button type="button" className="w-full" onClick={finish}>
            {t('email.continue')}
          </Button>
          <Button type="button" variant="ghost" className="w-full" onClick={finish}>
            {t('email.skip')}
          </Button>
        </div>
      </div>
    </AuthCard>
  )
}
