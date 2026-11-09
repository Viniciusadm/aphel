import { LayoutGrid } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useInstalledBlueprint } from './blueprint'
import { useNavigationEntries } from './navigation'
import { useModuleState } from './registry'

const HOME_PATH = '/'
const MODULES_PATH = '/integrations/modules'
const SETTINGS_PATH = '/settings'

export function HomeFallback() {
  const { t } = useTranslation('layout')
  const state = useModuleState()
  const entries = useNavigationEntries()
  const blueprint = useInstalledBlueprint()
  if (!state.loaded || blueprint.isPending) return <Skeleton className="h-40 w-full" />
  const home = blueprint.data?.home
  if (home && home !== HOME_PATH && entries.some((entry) => entry.to === home)) {
    return <Navigate to={home} replace />
  }
  const settings = entries.find((entry) => entry.to === SETTINGS_PATH)
  return (
    <EmptyState
      icon={LayoutGrid}
      title={t('home.blankTitle')}
      description={t('home.blankDescription')}
      action={
        settings ? (
          <div className="flex flex-wrap justify-center gap-2">
            <Button asChild size="sm">
              <Link to={MODULES_PATH}>{t('home.openModules')}</Link>
            </Button>
            <Button asChild size="sm" variant="outline">
              <Link to={SETTINGS_PATH}>{t('home.openSettings')}</Link>
            </Button>
          </div>
        ) : undefined
      }
    />
  )
}
