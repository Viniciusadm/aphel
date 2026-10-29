import { Navigate, Outlet, useSearchParams } from 'react-router'
import { useSessionQuery } from '@/app/session/session-query'
import { REDIRECT_PARAM, safeRedirectPath } from '../lib/redirect'
import { FullPageLoader } from './FullPageLoader'

export function GuestOnly() {
  const [searchParams] = useSearchParams()
  const { data, isPending } = useSessionQuery()

  if (data) return <Navigate to={safeRedirectPath(searchParams.get(REDIRECT_PARAM))} replace />
  if (isPending) return <FullPageLoader />
  return <Outlet />
}
