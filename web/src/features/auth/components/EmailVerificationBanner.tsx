import { MailWarning } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { ResendVerificationButton } from './ResendVerificationButton'

export function EmailVerificationBanner() {
  const { t } = useTranslation('auth')
  const { me } = useSession()

  if (me.user.email_verified) return null

  return (
    <div
      role="status"
      className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b bg-muted/60 px-4 py-2 text-sm text-muted-foreground"
    >
      <MailWarning className="size-4 shrink-0" aria-hidden />
      <span className="flex-1">{t('emailVerification.message', { email: me.user.email })}</span>
      <ResendVerificationButton variant="ghost" />
    </div>
  )
}
