import { Navigate, Outlet, useLocation } from 'react-router'
import { FullPageLoader } from '@/features/auth/components/FullPageLoader'
import { useInstallerAvailable } from './api'

const GATED_PATHS = ['/login']

export function InstallerGate() {
  const { pathname } = useLocation()
  const gated = GATED_PATHS.includes(pathname)
  const { data: available, isPending } = useInstallerAvailable()

  if (gated && isPending) return <FullPageLoader />
  if (gated && available) return <Navigate to="/install" replace />
  return <Outlet />
}
