import { useState, type ReactNode } from 'react'
import { Navigate, Outlet, useLocation } from 'react-router'
import { ErrorState } from '@/components/ui/error-state'
import { FullPageLoader } from '@/features/auth/components/FullPageLoader'
import { loginPathFor } from '@/features/auth/lib/redirect'
import { ModulesSync } from '@/kernel/modules/ModulesSync'
import { SessionProvider } from './SessionProvider'
import { useSessionQuery } from './session-query'

export function RequireAuth({ children }: { children?: ReactNode }) {
  const location = useLocation()
  const [signedOut, setSignedOut] = useState(false)
  const { data, isError, refetch } = useSessionQuery()

  if (data) {
    return (
      <SessionProvider me={data} onSignedOut={() => setSignedOut(true)}>
        <ModulesSync />
        {children ?? <Outlet />}
      </SessionProvider>
    )
  }

  if (data === null) {
    const target = signedOut ? '/login' : loginPathFor(`${location.pathname}${location.search}${location.hash}`)
    return <Navigate to={target} replace />
  }

  if (isError) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background p-4">
        <ErrorState className="w-full max-w-md" onRetry={() => void refetch()} />
      </div>
    )
  }

  return <FullPageLoader />
}
