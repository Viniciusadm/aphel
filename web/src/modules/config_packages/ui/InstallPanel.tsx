import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import {configPackagesApi, type PackagePlan, type PackageSource} from '@/sdk'
import { PlanView } from '@/modules/config_packages/ui/PlanView'
import { SourcePicker } from '@/modules/config_packages/ui/SourcePicker'

interface PlannedPackage {
  source: PackageSource
  plan: PackagePlan
}

interface InstallPanelProps {
  onInstalled: () => void
}

export function InstallPanel({ onInstalled }: InstallPanelProps) {
  const { t } = useTranslation('config-packages')
  const [planned, setPlanned] = useState<PlannedPackage | null>(null)
  const [planning, setPlanning] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)

  const requestPlan = async (source: PackageSource) => {
    setPlanning(true)
    try {
      const plan = await configPackagesApi.plan(source)
      setPlanned({ source, plan })
    } catch (error) {
      notifyError(error)
    } finally {
      setPlanning(false)
    }
  }

  const choose = (source: PackageSource) => {
    setNotice(null)
    void requestPlan(source)
  }

  const replanOutdated = async () => {
    if (!planned) return
    setNotice(t('plan.outdated'))
    await requestPlan(planned.source)
  }

  const reset = () => {
    setPlanned(null)
    setNotice(null)
  }

  if (planned) {
    return (
      <PlanView
        plan={planned.plan}
        source={planned.source}
        notice={notice}
        replanning={planning}
        onBack={reset}
        onOutdated={replanOutdated}
        onApplied={() => {
          reset()
          onInstalled()
        }}
      />
    )
  }

  return <SourcePicker planning={planning} onChoose={choose} />
}
