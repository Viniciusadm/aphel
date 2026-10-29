import { Outlet } from 'react-router'
import { FullPageLoader } from '@/features/auth/components/FullPageLoader'
import { AppLayout } from './layout/AppLayout'
import { RequireAuth } from './session/RequireAuth'
import { useSessionQuery } from './session/session-query'

export function UnknownRouteShell() {
  const { data, isPending } = useSessionQuery()

  if (data) {
    return (
      <RequireAuth>
        <AppLayout />
      </RequireAuth>
    )
  }

  if (isPending) return <FullPageLoader />

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <Outlet />
    </div>
  )
}
