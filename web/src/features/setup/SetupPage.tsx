import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useNavigate } from 'react-router'
import { SessionProvider } from '@/app/session/SessionProvider'
import { useSession } from '@/app/session/session-context'
import { useSessionQuery } from '@/app/session/session-query'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import type { SetupStepContribution } from '@/kernel/modules/define'
import { ModulesSync } from '@/kernel/modules/ModulesSync'
import { useActiveContributions, useModuleState, type Contribution } from '@/kernel/modules/registry'
import { SetupFrameContext, SetupStepFrame } from '@/kernel/setup'
import { useInstallerAvailable } from './api'
import { InstallWizard } from './InstallWizard'
import { SetupLoadingCard } from './components/SetupLoadingCard'
import { SetupProgress } from './components/SetupProgress'
import { SetupShell } from './components/SetupShell'
import { contributedStepKey, useStepLabel } from './lib/steps'
import { ReadyStep } from './steps/ReadyStep'

export function SetupPage() {
  const { t } = useTranslation('setup')
  const installer = useInstallerAvailable()
  const [installing, setInstalling] = useState(false)
  if (installer.data === true && !installing) setInstalling(true)

  if (installing) return <InstallWizard />
  if (installer.isPending) return <SetupLoading />
  if (installer.isError) {
    return (
      <SetupShell>
        <SetupStepFrame
          title={t('organization.title')}
          error={t('errors.statusFailed')}
          primary={{ label: t('retry'), onClick: () => void installer.refetch() }}
        />
      </SetupShell>
    )
  }
  return <ResumeSetup />
}

export function SetupLoading() {
  const { t } = useTranslation('setup')
  return (
    <SetupShell>
      <SetupLoadingCard label={t('loading')} />
    </SetupShell>
  )
}

function ResumeSetup() {
  const { t } = useTranslation('setup')
  const session = useSessionQuery()
  if (session.data === null) return <Navigate to="/login" replace />
  if (!session.data) {
    if (session.isError) {
      return (
        <SetupShell>
          <SetupStepFrame
            title={t('organization.title')}
            error={t('errors.statusFailed')}
            primary={{ label: t('retry'), onClick: () => void session.refetch() }}
          />
        </SetupShell>
      )
    }
    return <SetupLoading />
  }
  return (
    <SessionProvider me={session.data}>
      <ModulesSync />
      <ResumeWizard />
    </SessionProvider>
  )
}

function PendingProbe({ usePending, onResult }: { usePending: () => boolean | undefined; onResult: (pending: boolean | undefined) => void }) {
  const pending = usePending()
  useEffect(() => onResult(pending), [onResult, pending])
  return null
}

function ResumeWizard() {
  const { t } = useTranslation('setup')
  const navigate = useNavigate()
  const label = useStepLabel()
  const { hasPermission } = useSession()
  const moduleState = useModuleState()
  const contributed = useActiveContributions<SetupStepContribution>('setupSteps')
  const probed = contributed.filter((item) => item.value.usePending)
  const [pending, setPending] = useState<Record<string, boolean | undefined>>({})
  const [plan, setPlan] = useState<Contribution<SetupStepContribution>[] | null>(null)
  const [current, setCurrent] = useState(0)
  const resolved = moduleState.loaded && probed.every((item) => pending[contributedStepKey(item)] !== undefined)
  if (resolved && plan === null) setPlan(probed.filter((item) => pending[contributedStepKey(item)] === true))

  if (!hasPermission(SETTINGS_MANAGE)) return <Navigate to="/" replace />
  if (plan === null) {
    return (
      <>
        {probed.map((item) => (
          <PendingProbe
            key={contributedStepKey(item)}
            usePending={item.value.usePending as () => boolean | undefined}
            onResult={(value) =>
              setPending((state) => (state[contributedStepKey(item)] === value ? state : { ...state, [contributedStepKey(item)]: value }))
            }
          />
        ))}
        <SetupLoading />
      </>
    )
  }
  if (plan.length === 0) return <Navigate to="/" replace />

  const steps = [
    ...plan.map((item) => ({ key: contributedStepKey(item), label: label(item.value.label) })),
    { key: 'ready', label: t('steps.ready') },
  ]
  const active = plan[current]
  const Render = active?.value.render

  return (
    <SetupShell progress={<SetupProgress steps={steps} current={current} />}>
      <SetupFrameContext.Provider value={{}}>
        {Render ? (
          <Render key={steps[current].key} onDone={() => setCurrent((index) => index + 1)} />
        ) : (
          <ReadyStep onFinish={() => void navigate('/', { replace: true })} />
        )}
      </SetupFrameContext.Provider>
    </SetupShell>
  )
}
