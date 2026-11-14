import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate } from 'react-router'
import { SetupStepFrame } from '@/kernel/setup'
import { useInstallerAvailable } from './api'
import { InstallWizard } from './InstallWizard'
import { SetupLoadingCard } from './components/SetupLoadingCard'
import { SetupShell } from './components/SetupShell'

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
  return <Navigate to="/" replace />
}

function SetupLoading() {
  const { t } = useTranslation('setup')
  return (
    <SetupShell>
      <SetupLoadingCard label={t('loading')} />
    </SetupShell>
  )
}
