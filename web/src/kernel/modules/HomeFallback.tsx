import { LayoutGrid } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Navigate } from 'react-router'
import { Skeleton } from '@/components/ui/skeleton'
import { EmptyState } from '@/components/ui/empty-state'
import { SETTINGS_ENTRY_KEY, useNavigationEntries } from './navigation'
import { useModuleState } from './registry'

const HOME_PATH = '/'

export function HomeFallback() {
  const { t } = useTranslation('layout')
  const state = useModuleState()
  const entries = useNavigationEntries()
  if (!state.loaded) return <Skeleton className="h-40 w-full" />
  const target =
    entries.find((entry) => entry.to !== HOME_PATH && entry.key !== SETTINGS_ENTRY_KEY) ??
    entries.find((entry) => entry.to !== HOME_PATH)
  if (target) return <Navigate to={target.to} replace />
  return <EmptyState icon={LayoutGrid} title={t('home.emptyTitle')} description={t('home.emptyDescription')} />
}
