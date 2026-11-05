import { ArrowRight, Rocket, X } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { SETTINGS_MANAGE } from '@/sdk'
import { useOnboardingState } from './api'
import { markPrompted, wasPrompted } from './lib/prompt'

export function OnboardingGate() {
  const { me, hasPermission } = useSession()
  if (!hasPermission(SETTINGS_MANAGE)) return null
  return <ResumeSetupNotice userId={me.user.id} />
}

function ResumeSetupNotice({ userId }: { userId: string }) {
  const { t } = useTranslation('onboarding')
  const [dismissed, setDismissed] = useState(() => wasPrompted(userId))
  const { data } = useOnboardingState(!dismissed)
  if (dismissed || data?.status !== 'pending') return null

  const dismiss = () => {
    markPrompted(userId)
    setDismissed(true)
  }

  return (
    <aside
      aria-label={t('resume.title')}
      className="fixed right-4 bottom-[calc(5.5rem+env(safe-area-inset-bottom))] left-4 z-40 flex items-start gap-3 rounded-xl border bg-popover p-4 text-popover-foreground shadow-lg sm:left-auto sm:w-96 md:bottom-4"
    >
      <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
        <Rocket className="size-4" aria-hidden />
      </span>
      <div className="grid min-w-0 flex-1 gap-2">
        <div className="grid gap-0.5">
          <p className="text-sm font-semibold">{t('resume.title')}</p>
          <p className="text-sm text-muted-foreground">{t('resume.description')}</p>
        </div>
        <Button asChild size="sm" className="justify-self-start">
          <Link to="/setup">
            {t('resume.continue')}
            <ArrowRight aria-hidden />
          </Link>
        </Button>
      </div>
      <Button type="button" variant="ghost" size="icon" className="-mt-1 -mr-1 size-8" onClick={dismiss} aria-label={t('resume.dismiss')}>
        <X aria-hidden />
      </Button>
    </aside>
  )
}
