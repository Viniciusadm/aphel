import { CircleAlert } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { AuthCard } from './AuthCard'

interface InvalidTokenNoticeProps {
  description: string
  action?: ReactNode
}

export function InvalidTokenNotice({ description, action }: InvalidTokenNoticeProps) {
  const { t } = useTranslation('auth')
  return (
    <AuthCard title={t('invalidToken.title')}>
      <div role="alert" className="grid gap-4">
        <p className="flex items-start gap-2 text-sm text-muted-foreground">
          <CircleAlert className="mt-0.5 size-4 shrink-0 text-destructive" aria-hidden />
          <span>{description}</span>
        </p>
        {action}
      </div>
    </AuthCard>
  )
}
