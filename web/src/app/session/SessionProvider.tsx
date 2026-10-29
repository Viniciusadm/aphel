import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useMemo, type ReactNode } from 'react'
import { isApiError } from '@/api'
import { runSignOutCleanups } from '@/pwa/sign-out'
import { logout as logoutRequest, logoutAll as logoutAllRequest } from '@/features/auth/api'
import { hasPermissionOf, permissionScopeOf } from './permissions'
import { SessionContext, type Me, type SessionContextValue } from './session-context'
import { clearSession } from './session-query'

interface SessionProviderProps {
  me: Me
  onSignedOut?: () => void
  children: ReactNode
}

export function SessionProvider({ me, onSignedOut, children }: SessionProviderProps) {
  const queryClient = useQueryClient()

  const signOut = useCallback(
    async (request: () => Promise<void>) => {
      try {
        await request()
      } catch (error) {
        if (!isApiError(error) || error.status !== 401) throw error
      }
      await runSignOutCleanups()
      onSignedOut?.()
      clearSession(queryClient)
    },
    [onSignedOut, queryClient],
  )

  const value = useMemo<SessionContextValue>(
    () => ({
      me,
      hasPermission: (permission) => hasPermissionOf(me, permission),
      permissionScope: (permission) => permissionScopeOf(me, permission),
      logout: () => signOut(logoutRequest),
      logoutAll: () => signOut(logoutAllRequest),
    }),
    [me, signOut],
  )

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>
}
