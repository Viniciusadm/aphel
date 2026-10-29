import { Outlet, useLocation } from 'react-router'
import { Skeleton } from '@/components/ui/skeleton'
import { NotFoundPage } from '@/features/not-found/NotFoundPage'
import { HomeFallback } from './HomeFallback'
import { isModuleActive, useModuleState } from './registry'

export function ModuleGate({ moduleKey }: { moduleKey: string }) {
  const state = useModuleState()
  const { pathname } = useLocation()
  if (!state.loaded) return <Skeleton className="h-40 w-full" />
  if (!isModuleActive(moduleKey, state)) return pathname === '/' ? <HomeFallback /> : <NotFoundPage />
  return <Outlet />
}
