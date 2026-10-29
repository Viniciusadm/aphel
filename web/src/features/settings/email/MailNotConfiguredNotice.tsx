import { MailWarning } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { useOrganizationConfiguration } from '../general/api'
import { SETTINGS_MANAGE } from '../lib/permissions'

export const OUTGOING_EMAIL_PATH = '/settings/outgoing-email'

export function MailNotConfiguredNotice({ showLink = true }: { showLink?: boolean }) {
  const { t } = useTranslation('settings')
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const configuration = useOrganizationConfiguration(canManage)
  if (!canManage || !configuration.data || configuration.data.mail_configured) return null
  return (
    <div role="status" className="flex items-start gap-3 rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
      <MailWarning className="mt-0.5 size-4 shrink-0" aria-hidden />
      <div className="grid gap-1">
        <p className="font-medium">{t('configuration.mail.notConfiguredTitle')}</p>
        <p className="text-muted-foreground">{t('configuration.mail.notConfiguredDescription')}</p>
        {showLink && (
          <Link to={OUTGOING_EMAIL_PATH} className="font-medium underline underline-offset-4">
            {t('configuration.mail.configure')}
          </Link>
        )}
      </div>
    </div>
  )
}
